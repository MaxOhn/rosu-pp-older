use std::{
    cmp,
    fmt::{Debug, Formatter, Result as FmtResult},
    num::NonZeroU32,
    pin::Pin,
};

use convert::convert_objects;
use difficulty_object::OsuDifficultyObject;
use osu_object::OsuObject;
use rosu_map::util::Pos;
use rosu_pp::{
    model::{beatmap::BeatmapAttributes, mode::GameMode},
    Beatmap,
};
use scaling_factor::ScalingFactor;
use skills::OsuSkills;

pub use self::{
    attributes::{OsuDifficultyAttributes, OsuPerformanceAttributes},
    pp::*,
};

use crate::util::{mods::Mods, skills::Skill};

mod attributes;
mod convert;
mod difficulty_object;
mod osu_object;
mod pp;
mod scaling_factor;
mod skills;

const PLAYFIELD_BASE_SIZE: Pos = Pos::new(512.0, 384.0);

const DIFFICULTY_MULTIPLIER: f64 = 0.0675;

const HD_FADE_IN_DURATION_MULTIPLIER: f64 = 0.4;
const HD_FADE_OUT_DURATION_MULTIPLIER: f64 = 0.3;

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
pub struct OsuStars {
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

impl OsuStars {
    /// Create a new difficulty calculator.
    pub const fn new() -> Self {
        Self {
            mods: 0,
            passed_objects: None,
            clock_rate: None,
        }
    }

    /// Specify mods.
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
    pub fn calculate(&self, map: &Beatmap) -> OsuDifficultyAttributes {
        let Ok(map) = map.convert_ref(GameMode::Osu, &self.mods.into()) else {
            return Default::default();
        };

        let map = map.as_ref();

        let DifficultyValues {
            skills:
                OsuSkills {
                    aim,
                    aim_no_sliders,
                    speed,
                    flashlight,
                },
            mut attrs,
        } = DifficultyValues::calculate(self, map);

        let aim_difficulty_value = aim.difficulty_value();
        let aim_no_sliders_difficulty_value = aim_no_sliders.difficulty_value();
        let speed_relevant_note_count = speed.relevant_note_count();
        let speed_difficulty_value = speed.difficulty_value();
        let flashlight_difficulty_value = flashlight.difficulty_value();

        let mods = self.get_mods();

        DifficultyValues::eval(
            &mut attrs,
            mods,
            aim_difficulty_value,
            aim_no_sliders_difficulty_value,
            speed_difficulty_value,
            speed_relevant_note_count,
            flashlight_difficulty_value,
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

impl Debug for OsuStars {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let Self {
            mods,
            passed_objects,
            clock_rate,
        } = self;

        f.debug_struct("OsuStars")
            .field("mods", mods)
            .field("passed_objects", passed_objects)
            .field("clock_rate", &clock_rate.map(non_zero_u32_to_f32))
            .finish()
    }
}

impl Default for OsuStars {
    fn default() -> Self {
        Self::new()
    }
}

pub struct OsuDifficultySetup {
    scaling_factor: ScalingFactor,
    map_attrs: BeatmapAttributes,
    attrs: OsuDifficultyAttributes,
    time_preempt: f64,
}

impl OsuDifficultySetup {
    pub fn new(difficulty: &OsuStars, map: &Beatmap) -> Self {
        let clock_rate = difficulty.get_clock_rate();
        let map_attrs = map.attributes().mods(difficulty.get_mods()).build();
        let scaling_factor = ScalingFactor::new(map_attrs.cs);

        let attrs = OsuDifficultyAttributes {
            ar: map_attrs.ar,
            hp: map_attrs.hp,
            od: map_attrs.od,
            ..Default::default()
        };

        let time_preempt = f64::from((map_attrs.hit_windows.ar * clock_rate) as f32);

        Self {
            scaling_factor,
            map_attrs,
            attrs,
            time_preempt,
        }
    }
}

pub struct DifficultyValues {
    pub skills: OsuSkills,
    pub attrs: OsuDifficultyAttributes,
}

impl DifficultyValues {
    pub fn calculate(difficulty: &OsuStars, map: &Beatmap) -> Self {
        let mods = difficulty.get_mods();
        let take = difficulty.get_passed_objects();

        let OsuDifficultySetup {
            scaling_factor,
            map_attrs,
            mut attrs,
            time_preempt,
        } = OsuDifficultySetup::new(difficulty, map);

        let mut osu_objects = convert_objects(
            map,
            &scaling_factor,
            mods.hr(),
            time_preempt,
            take,
            &mut attrs,
        );

        let osu_object_iter = osu_objects.iter_mut().map(Pin::new);

        let diff_objects =
            Self::create_difficulty_objects(difficulty, &scaling_factor, osu_object_iter);

        let mut skills = OsuSkills::new(mods, &scaling_factor, &map_attrs, time_preempt);

        {
            let mut aim = Skill::new(&mut skills.aim, &diff_objects);
            let mut aim_no_sliders = Skill::new(&mut skills.aim_no_sliders, &diff_objects);
            let mut speed = Skill::new(&mut skills.speed, &diff_objects);
            let mut flashlight = Skill::new(&mut skills.flashlight, &diff_objects);

            // The first hit object has no difficulty object
            let take_diff_objects = cmp::min(map.hit_objects.len(), take).saturating_sub(1);

            for hit_object in diff_objects.iter().take(take_diff_objects) {
                aim.process(hit_object);
                aim_no_sliders.process(hit_object);
                speed.process(hit_object);
                flashlight.process(hit_object);
            }
        }

        Self { skills, attrs }
    }

    /// Process the difficulty values and store the results in `attrs`.
    pub fn eval(
        attrs: &mut OsuDifficultyAttributes,
        mods: u32,
        aim_difficulty_value: f64,
        aim_no_sliders_difficulty_value: f64,
        speed_difficulty_value: f64,
        speed_relevant_note_count: f64,
        flashlight_difficulty_value: f64,
    ) {
        let mut aim_rating = aim_difficulty_value.sqrt() * DIFFICULTY_MULTIPLIER;
        let aim_rating_no_sliders = aim_no_sliders_difficulty_value.sqrt() * DIFFICULTY_MULTIPLIER;
        let mut speed_rating = speed_difficulty_value.sqrt() * DIFFICULTY_MULTIPLIER;
        let mut flashlight_rating = flashlight_difficulty_value.sqrt() * DIFFICULTY_MULTIPLIER;

        let slider_factor = if aim_rating > 0.0 {
            aim_rating_no_sliders / aim_rating
        } else {
            1.0
        };

        if mods.td() {
            aim_rating = aim_rating.powf(0.8);
            flashlight_rating = flashlight_rating.powf(0.8);
        }

        if mods.rx() {
            aim_rating *= 0.9;
            speed_rating = 0.0;
            flashlight_rating *= 0.7;
        }

        let base_aim_performance =
            (5.0 * (aim_rating / 0.0675).max(1.0) - 4.0).powf(3.0) / 100_000.0;
        let base_speed_performance =
            (5.0 * (speed_rating / 0.0675).max(1.0) - 4.0).powf(3.0) / 100_000.0;

        let base_flashlight_performance = if mods.fl() {
            flashlight_rating.powf(2.0) * 25.0
        } else {
            0.0
        };

        let base_performance = ((base_aim_performance).powf(1.1)
            + (base_speed_performance).powf(1.1)
            + (base_flashlight_performance).powf(1.1))
        .powf(1.0 / 1.1);

        let star_rating = if base_performance > 0.00001 {
            PERFORMANCE_BASE_MULTIPLIER.cbrt()
                * 0.027
                * ((100_000.0 / 2.0_f64.powf(1.0 / 1.1) * base_performance).cbrt() + 4.0)
        } else {
            0.0
        };

        attrs.aim = aim_rating;
        attrs.speed = speed_rating;
        attrs.flashlight = flashlight_rating;
        attrs.slider_factor = slider_factor;
        attrs.stars = star_rating;
        attrs.speed_note_count = speed_relevant_note_count;
    }

    pub fn create_difficulty_objects<'a>(
        difficulty: &OsuStars,
        scaling_factor: &ScalingFactor,
        osu_objects: impl ExactSizeIterator<Item = Pin<&'a mut OsuObject>>,
    ) -> Vec<OsuDifficultyObject<'a>> {
        let take = difficulty.get_passed_objects();
        let clock_rate = difficulty.get_clock_rate();

        let mut osu_objects_iter = osu_objects
            .map(|h| OsuDifficultyObject::compute_slider_cursor_pos(h, scaling_factor.radius))
            .map(Pin::into_ref);

        let Some(mut last) = osu_objects_iter.next().filter(|_| take > 0) else {
            return Vec::new();
        };

        let mut last_last = None;

        osu_objects_iter
            .enumerate()
            .map(|(idx, h)| {
                let diff_object = OsuDifficultyObject::new(
                    h.get_ref(),
                    last.get_ref(),
                    last_last.as_deref(),
                    clock_rate,
                    idx,
                    scaling_factor,
                );

                last_last = Some(last);
                last = h;

                diff_object
            })
            .collect()
    }
}
