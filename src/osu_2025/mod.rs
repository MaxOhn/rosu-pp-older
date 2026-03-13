use rosu_map::util::Pos;
use rosu_pp::{
    model::mode::{ConvertError, GameMode, IGameMode},
    osu::OsuHitResults,
    Beatmap, Difficulty,
};

pub use self::{
    attributes::{OsuDifficultyAttributes, OsuPerformanceAttributes},
    performance::OsuPerformance as OsuPP,
    score_state::OsuScoreState,
};

use crate::any::difficulty::DifficultyExt;

mod attributes;
mod convert;
mod difficulty;
mod object;
mod performance;
mod score_state;

const PLAYFIELD_BASE_SIZE: Pos = Pos::new(512.0, 384.0);

pub struct Osu25;

impl IGameMode for Osu25 {
    type DifficultyAttributes = OsuDifficultyAttributes;
    type Strains = ();
    type Performance<'map> = OsuPP<'map>;
    type HitResults = OsuHitResults;
    type GradualDifficulty = ();
    type GradualPerformance = ();

    fn difficulty(_: &Difficulty, _: &Beatmap) -> Result<Self::DifficultyAttributes, ConvertError> {
        unimplemented!()
    }

    fn strains(_: &Difficulty, _: &Beatmap) -> Result<Self::Strains, ConvertError> {
        unimplemented!()
    }

    fn performance(_: &Beatmap) -> Self::Performance<'_> {
        unimplemented!()
    }

    fn gradual_difficulty(
        _: Difficulty,
        _: &Beatmap,
    ) -> Result<Self::GradualDifficulty, ConvertError> {
        unimplemented!()
    }

    fn gradual_performance(
        _: Difficulty,
        _: &Beatmap,
    ) -> Result<Self::GradualPerformance, ConvertError> {
        unimplemented!()
    }
}

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
    pub fn calculate(&self, map: &Beatmap) -> OsuDifficultyAttributes {
        Self::calculate_static(&self.difficulty, map)
    }

    pub(crate) fn calculate_static(
        difficulty: &Difficulty,
        map: &Beatmap,
    ) -> OsuDifficultyAttributes {
        let Ok(map) = map.convert_ref(GameMode::Osu, &difficulty.get_mods()) else {
            return Default::default();
        };

        difficulty::difficulty(difficulty, &map).unwrap_or_default()
    }
}
