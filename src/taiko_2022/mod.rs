use std::{
    fmt::{Debug, Formatter, Result as FmtResult},
    num::NonZeroU32,
};

use color::preprocessor::ColorDifficultyPreprocessor;
use difficulty_object::{TaikoDifficultyObject, TaikoDifficultyObjects};
use rosu_pp::{model::mode::GameMode, Beatmap};
use skills::peaks::{Peaks, PeaksSkill};
use taiko_object::TaikoObject;

use crate::util::mods::Mods;

pub use self::{
    attributes::{TaikoDifficultyAttributes, TaikoPerformanceAttributes},
    pp::*,
};

mod attributes;
mod color;
mod difficulty_object;
mod pp;
mod rhythm;
mod skills;
mod taiko_object;

/// Difficulty calculator on maps of any mode.
///
/// # Example
///
/// ```
/// use rosu_pp::{Beatmap, Difficulty, any::DifficultyAttributes};
///
/// let map = Beatmap::from_path("./resources/2118524.osu").unwrap();
///
/// let attrs: DifficultyAttributes = Difficulty::new()
///     .mods(8 + 1024) // HDFL
///     .calculate(&map);
/// ```
#[derive(Clone, PartialEq)]
#[must_use]
pub struct TaikoStars {
    mods: u32,
    passed_objects: Option<u32>,
    /// Clock rate will be clamped internally between 0.01 and 100.0.
    ///
    /// Since its minimum value is 0.01, its bits are never zero.
    /// Additionally, values between 0.01 and 100 are represented sufficiently
    /// precise with 32 bits.
    ///
    /// This allows for an optimization to reduce the struct size by storing its
    /// bits as a [`NonZeroU32`].
    clock_rate: Option<NonZeroU32>,
}

impl TaikoStars {
    /// Create a new difficulty calculator.
    pub fn new() -> Self {
        Self {
            mods: 0,
            passed_objects: None,
            clock_rate: None,
        }
    }

    /// Specify mods.
    ///
    /// Accepted types are
    /// - `u32`
    /// - [`rosu_mods::GameModsLegacy`]
    /// - [`rosu_mods::GameMods`]
    /// - [`rosu_mods::GameModsIntermode`]
    /// - [`&rosu_mods::GameModsIntermode`](rosu_mods::GameModsIntermode)
    ///
    /// See <https://github.com/ppy/osu-api/wiki#mods>
    pub const fn mods(self, mods: u32) -> Self {
        Self { mods, ..self }
    }

    /// Amount of passed objects for partial plays, e.g. a fail.
    pub const fn passed_objects(mut self, passed_objects: u32) -> Self {
        self.passed_objects = Some(passed_objects);

        self
    }

    /// Adjust the clock rate used in the calculation.
    ///
    /// If none is specified, it will take the clock rate based on the mods
    /// i.e. 1.5 for DT, 0.75 for HT and 1.0 otherwise.
    ///
    /// | Minimum | Maximum |
    /// | :-----: | :-----: |
    /// | 0.01    | 100     |
    pub fn clock_rate(self, clock_rate: f64) -> Self {
        let clock_rate = (clock_rate as f32).clamp(0.01, 100.0).to_bits();

        // SAFETY: The minimum value is 0.01 so its bits can never be fully
        // zero.
        let non_zero = unsafe { NonZeroU32::new_unchecked(clock_rate) };

        Self {
            clock_rate: Some(non_zero),
            ..self
        }
    }

    /// Perform the difficulty calculation.
    pub fn calculate(&self, map: &Beatmap) -> TaikoDifficultyAttributes {
        let Ok(map) = map.convert_ref(GameMode::Taiko, &self.mods.into()) else {
            return Default::default();
        };

        let map = map.as_ref();

        let hit_window = map
            .attributes()
            .mods(self.get_mods())
            .hit_windows()
            .od_great;

        let DifficultyValues { peaks, max_combo } = DifficultyValues::calculate(self, map);

        let mut attrs = TaikoDifficultyAttributes {
            hit_window,
            max_combo,
            is_convert: map.is_convert,
            ..Default::default()
        };

        let color_rating = peaks.color_difficulty_value();
        let rhythm_rating = peaks.rhythm_difficulty_value();
        let stamina_rating = peaks.stamina_difficulty_value();
        let combined_rating = peaks.difficulty_value();

        DifficultyValues::eval(
            &mut attrs,
            color_rating,
            rhythm_rating,
            stamina_rating,
            combined_rating,
        );

        attrs
    }

    pub(crate) const fn get_mods(&self) -> u32 {
        self.mods
    }

    pub(crate) fn get_clock_rate(&self) -> f64 {
        let clock_rate = self
            .clock_rate
            .map_or(self.mods.clock_rate() as f32, non_zero_u32_to_f32);

        f64::from(clock_rate)
    }

    pub(crate) fn get_passed_objects(&self) -> usize {
        self.passed_objects.map_or(usize::MAX, |n| n as usize)
    }
}

fn non_zero_u32_to_f32(n: NonZeroU32) -> f32 {
    f32::from_bits(n.get())
}

impl Debug for TaikoStars {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let Self {
            mods,
            passed_objects,
            clock_rate,
        } = self;

        f.debug_struct("TaikoStars")
            .field("mods", mods)
            .field("passed_objects", passed_objects)
            .field("clock_rate", &clock_rate.map(non_zero_u32_to_f32))
            .finish()
    }
}

impl Default for TaikoStars {
    fn default() -> Self {
        Self::new()
    }
}

fn rescale(stars: f64) -> f64 {
    if stars < 0.0 {
        stars
    } else {
        10.43 * (stars / 8.0 + 1.0).ln()
    }
}

pub struct DifficultyValues {
    pub peaks: Peaks,
    pub max_combo: u32,
}

impl DifficultyValues {
    pub fn calculate(difficulty: &TaikoStars, map: &Beatmap) -> Self {
        let take = difficulty.get_passed_objects();
        let clock_rate = difficulty.get_clock_rate();

        let mut n_diff_objects = 0;
        let mut max_combo = 0;

        let diff_objects = Self::create_difficulty_objects(
            map,
            take as u32,
            clock_rate,
            &mut max_combo,
            &mut n_diff_objects,
        );

        // The first two hit objects have no difficulty object
        n_diff_objects = n_diff_objects.saturating_sub(2);

        let mut peaks = Peaks::new();

        {
            let mut peaks = PeaksSkill::new(&mut peaks, &diff_objects);

            for hit_object in diff_objects.iter().take(n_diff_objects) {
                peaks.process(&hit_object.get());
            }
        }

        Self { peaks, max_combo }
    }

    pub fn eval(
        attrs: &mut TaikoDifficultyAttributes,
        color_difficulty_value: f64,
        rhythm_difficulty_value: f64,
        stamina_difficulty_value: f64,
        peaks_difficulty_value: f64,
    ) {
        const DIFFICULTY_MULTIPLIER: f64 = 1.35;

        let color_rating = color_difficulty_value * DIFFICULTY_MULTIPLIER;
        let rhythm_rating = rhythm_difficulty_value * DIFFICULTY_MULTIPLIER;
        let stamina_rating = stamina_difficulty_value * DIFFICULTY_MULTIPLIER;
        let combined_rating = peaks_difficulty_value * DIFFICULTY_MULTIPLIER;

        let mut star_rating = rescale(combined_rating * 1.4);

        // * TODO: This is temporary measure as we don't detect abuse of multiple-input
        // * playstyles of converts within the current system.
        if attrs.is_convert {
            star_rating *= 0.925;

            // * For maps with low colour variance and high stamina requirement,
            // * multiple inputs are more likely to be abused.
            if color_rating < 2.0 && stamina_rating > 8.0 {
                star_rating *= 0.8;
            }
        }

        attrs.stamina = stamina_rating;
        attrs.rhythm = rhythm_rating;
        attrs.color = color_rating;
        attrs.peak = combined_rating;
        attrs.stars = star_rating;
    }

    pub fn create_difficulty_objects(
        map: &Beatmap,
        take: u32,
        clock_rate: f64,
        max_combo: &mut u32,
        n_diff_objects: &mut usize,
    ) -> TaikoDifficultyObjects {
        let mut hit_objects_iter = map
            .hit_objects
            .iter()
            .zip(map.hit_sounds.iter())
            .map(|(h, s)| TaikoObject::new(h, *s))
            .inspect(|h| {
                if *max_combo < take {
                    *n_diff_objects += 1;
                    *max_combo += u32::from(h.is_hit());
                }
            });

        let Some((mut last_last, mut last)) = hit_objects_iter.next().zip(hit_objects_iter.next())
        else {
            return TaikoDifficultyObjects::with_capacity(0);
        };

        let mut diff_objects = TaikoDifficultyObjects::with_capacity(map.hit_objects.len() - 2);

        for (i, curr) in hit_objects_iter.enumerate() {
            let diff_object = TaikoDifficultyObject::new(
                &curr,
                &last,
                &last_last,
                clock_rate,
                i,
                &mut diff_objects,
            );

            diff_objects.push(diff_object);

            last_last = last;
            last = curr;
        }

        ColorDifficultyPreprocessor::process_and_assign(&diff_objects);

        diff_objects
    }
}
