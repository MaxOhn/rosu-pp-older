use rosu_pp::{model::mode::GameMode, Beatmap, Difficulty};

use crate::any::difficulty::DifficultyExt;

pub use self::{
    attributes::{CatchDifficultyAttributes, CatchPerformanceAttributes},
    performance::CatchPerformance as FruitsPP,
};

mod attributes;
mod catcher;
mod convert;
mod difficulty;
mod object;
mod performance;
mod score_state;

const PLAYFIELD_WIDTH: f32 = 512.0;

#[derive(Clone, PartialEq)]
#[must_use]
pub struct OsuStars {
    difficulty: Difficulty,
}

impl OsuStars {
    /// Create a new difficulty calculator.
    #[allow(clippy::new_without_default)]
    pub fn new() -> Self {
        Self {
            difficulty: Difficulty::new(),
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
    pub fn mods(self, mods: u32) -> Self {
        Self {
            difficulty: self.difficulty.mods(mods),
        }
    }

    /// Amount of passed objects for partial plays, e.g. a fail.
    pub fn passed_objects(self, passed_objects: u32) -> Self {
        Self {
            difficulty: self.difficulty.passed_objects(passed_objects),
        }
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
        Self {
            difficulty: self.difficulty.clock_rate(clock_rate),
        }
    }

    /// Perform the difficulty calculation.
    pub fn calculate(&self, map: &Beatmap) -> CatchDifficultyAttributes {
        Self::calculate_static(&self.difficulty, map)
    }

    pub(crate) fn calculate_static(
        difficulty: &Difficulty,
        map: &Beatmap,
    ) -> CatchDifficultyAttributes {
        let Ok(map) = map.convert_ref(GameMode::Osu, &difficulty.get_mods()) else {
            return Default::default();
        };

        difficulty::difficulty(difficulty, &map).unwrap_or_default()
    }
}
