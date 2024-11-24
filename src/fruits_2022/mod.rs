use std::{
    fmt::{Debug, Formatter, Result as FmtResult},
    num::NonZeroU32,
};

use attributes::ObjectCountBuilder;
use catch_object::palpable::PalpableObject;
use catcher::Catcher;
use convert::convert_objects;
use difficulty_object::CatchDifficultyObject;
use movement::Movement;
use rosu_pp::{
    model::{beatmap::BeatmapAttributes, mode::GameMode},
    Beatmap,
};

use crate::util::{mods::Mods, skills::Skill};

pub use self::{
    attributes::{CatchDifficultyAttributes, CatchPerformanceAttributes},
    pp::*,
};

mod attributes;
mod catch_object;
mod catcher;
mod convert;
mod difficulty_object;
mod movement;
mod pp;

const PLAYFIELD_WIDTH: f32 = 512.0;

const STAR_SCALING_FACTOR: f64 = 0.153;

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
pub struct CatchStars {
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

impl CatchStars {
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
    pub fn calculate(&self, map: &Beatmap) -> CatchDifficultyAttributes {
        let Ok(map) = map.convert_ref(GameMode::Catch, &self.mods.into()) else {
            return Default::default();
        };

        let map = map.as_ref();

        let DifficultyValues {
            movement,
            mut attrs,
        } = DifficultyValues::calculate(self, map);

        DifficultyValues::eval(&mut attrs, movement.difficulty_value());

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

impl Debug for CatchStars {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        let Self {
            mods,
            passed_objects,
            clock_rate,
        } = self;

        f.debug_struct("CatchStars")
            .field("mods", mods)
            .field("passed_objects", passed_objects)
            .field("clock_rate", &clock_rate.map(non_zero_u32_to_f32))
            .finish()
    }
}

impl Default for CatchStars {
    fn default() -> Self {
        Self::new()
    }
}

pub struct CatchDifficultySetup {
    map_attrs: BeatmapAttributes,
    attrs: CatchDifficultyAttributes,
}

impl CatchDifficultySetup {
    pub fn new(difficulty: &CatchStars, map: &Beatmap) -> Self {
        let map_attrs = map.attributes().mods(difficulty.get_mods()).build();

        let attrs = CatchDifficultyAttributes {
            ar: map_attrs.ar,
            is_convert: map.is_convert,
            ..Default::default()
        };

        Self { map_attrs, attrs }
    }
}

pub struct DifficultyValues {
    pub movement: Movement,
    pub attrs: CatchDifficultyAttributes,
}

impl DifficultyValues {
    pub fn calculate(difficulty: &CatchStars, map: &Beatmap) -> Self {
        let take = difficulty.get_passed_objects();
        let clock_rate = difficulty.get_clock_rate();

        let CatchDifficultySetup {
            map_attrs,
            mut attrs,
        } = CatchDifficultySetup::new(difficulty, map);

        let hr_offsets = difficulty.get_mods().hr();
        let mut count = ObjectCountBuilder::new(take);

        let palpable_objects = convert_objects(map, &mut count, hr_offsets, map_attrs.cs as f32);

        let diff_objects = Self::create_difficulty_objects(
            &map_attrs,
            clock_rate,
            palpable_objects.iter().take(take),
        );

        let mut movement = Movement::new(clock_rate);

        {
            let mut movement = Skill::new(&mut movement, &diff_objects);

            for curr in diff_objects.iter() {
                movement.process(curr);
            }
        }

        attrs.set_object_count(&count.into_regular());

        Self { movement, attrs }
    }

    pub fn eval(attrs: &mut CatchDifficultyAttributes, movement_difficulty_value: f64) {
        attrs.stars = movement_difficulty_value.sqrt() * STAR_SCALING_FACTOR;
    }

    pub fn create_difficulty_objects<'a>(
        map_attrs: &BeatmapAttributes,
        clock_rate: f64,
        mut palpable_objects: impl ExactSizeIterator<Item = &'a PalpableObject>,
    ) -> Box<[CatchDifficultyObject]> {
        let Some(mut last_object) = palpable_objects.next() else {
            return Box::default();
        };

        let mut half_catcher_width = Catcher::calculate_catch_width(map_attrs.cs as f32) * 0.5;
        half_catcher_width *= 1.0 - ((map_attrs.cs as f32 - 5.5).max(0.0) * 0.0625);
        let scaling_factor =
            CatchDifficultyObject::NORMALIZED_HITOBJECT_RADIUS / half_catcher_width;

        palpable_objects
            .enumerate()
            .map(|(i, hit_object)| {
                let diff_object = CatchDifficultyObject::new(
                    hit_object,
                    last_object,
                    clock_rate,
                    scaling_factor,
                    i,
                );
                last_object = hit_object;

                diff_object
            })
            .collect()
    }
}
