use std::{
    cmp,
    fmt::{Debug, Formatter, Result as FmtResult},
    num::NonZeroU32,
};

use difficulty_object::ManiaDifficultyObject;
use mania_object::{ManiaObject, ObjectParams};
use rosu_pp::{model::mode::GameMode, Beatmap};
use strain::Strain;

use crate::util::{mods::Mods, skills::Skill};

pub use self::{
    attributes::{ManiaDifficultyAttributes, ManiaPerformanceAttributes},
    pp::*,
};

mod attributes;
mod difficulty_object;
mod mania_object;
mod pp;
mod strain;

#[derive(Clone, PartialEq)]
#[must_use]
pub struct ManiaStars {
    mods: u32,
    passed_objects: Option<u32>,
    clock_rate: Option<NonZeroU32>,
}

impl ManiaStars {
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
    pub fn calculate(&self, map: &Beatmap) -> ManiaDifficultyAttributes {
        const STAR_SCALING_FACTOR: f64 = 0.018;

        let Ok(map) = map.convert_ref(GameMode::Mania, &self.mods.into()) else {
            return ManiaDifficultyAttributes::default();
        };

        let difficulty = self;
        let map = map.as_ref();

        let n_objects = cmp::min(difficulty.get_passed_objects(), map.hit_objects.len()) as u32;

        let values = DifficultyValues::calculate(difficulty, map);

        let hit_window = map
            .attributes()
            .mods(difficulty.get_mods())
            .hit_windows()
            .od_great;

        ManiaDifficultyAttributes {
            stars: values.strain.difficulty_value() * STAR_SCALING_FACTOR,
            hit_window,
            max_combo: values.max_combo,
            n_objects,
            is_convert: map.is_convert,
        }
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

impl Default for ManiaStars {
    fn default() -> Self {
        Self::new()
    }
}

impl Debug for ManiaStars {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let Self {
            mods,
            passed_objects,
            clock_rate,
        } = self;

        f.debug_struct("ManiaStars")
            .field("mods", mods)
            .field("passed_objects", passed_objects)
            .field("clock_rate", &clock_rate.map(non_zero_u32_to_f32))
            .finish()
    }
}

pub struct DifficultyValues {
    pub strain: Strain,
    pub max_combo: u32,
}

impl DifficultyValues {
    pub fn calculate(difficulty: &ManiaStars, map: &Beatmap) -> Self {
        let take = difficulty.get_passed_objects();
        let total_columns = map.cs.round_ties_even().max(1.0);
        let clock_rate = difficulty.get_clock_rate();
        let mut params = ObjectParams::new(map);

        let mania_objects = map
            .hit_objects
            .iter()
            .map(|h| ManiaObject::new(h, total_columns, &mut params))
            .take(take);

        let diff_objects = Self::create_difficulty_objects(clock_rate, mania_objects);

        let mut strain = Strain::new(total_columns as usize);

        {
            let mut strain = Skill::new(&mut strain, &diff_objects);

            for curr in diff_objects.iter() {
                strain.process(curr);
            }
        }

        Self {
            strain,
            max_combo: params.into_max_combo(),
        }
    }

    pub fn create_difficulty_objects(
        clock_rate: f64,
        mut mania_objects: impl ExactSizeIterator<Item = ManiaObject>,
    ) -> Box<[ManiaDifficultyObject]> {
        let Some(first) = mania_objects.next() else {
            return Box::default();
        };

        let n_diff_objects = mania_objects.len();

        let diff_objects_iter = mania_objects.enumerate().scan(first, |last, (i, base)| {
            let diff_object = ManiaDifficultyObject::new(&base, last, clock_rate, i);
            *last = base;

            Some(diff_object)
        });

        let mut diff_objects = Vec::with_capacity(n_diff_objects);
        diff_objects.extend(diff_objects_iter);

        debug_assert_eq!(n_diff_objects, diff_objects.len());

        diff_objects.into_boxed_slice()
    }
}
