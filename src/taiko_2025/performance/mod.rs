use std::cmp;

use rosu_pp::{
    any::{
        hitresult_generator::Fast, HitResultGenerator, HitResultPriority, InspectablePerformance,
    },
    model::mode::ConvertError,
    taiko::TaikoHitResults,
    Beatmap, Difficulty, GameMods,
};

use self::calculator::TaikoPerformanceCalculator;

pub use self::inspect::InspectTaikoPerformance;

use crate::{
    any::difficulty::DifficultyExt,
    taiko_2025::{Taiko25, TaikoDifficultyAttributes},
    util::map_or_attrs::MapOrAttrs,
};

use super::{attributes::TaikoPerformanceAttributes, score_state::TaikoScoreState};

mod calculator;
mod hitresult_generator;
mod inspect;

/// Performance calculator on osu!taiko maps.
#[derive(Clone, Debug)]
#[must_use]
pub struct TaikoPerformance<'map> {
    pub(crate) map_or_attrs: MapOrAttrs<'map, TaikoDifficultyAttributes>,
    difficulty: Difficulty,
    combo: Option<u32>,
    acc: Option<f64>,
    n300: Option<u32>,
    n100: Option<u32>,
    misses: Option<u32>,
    hitresult_priority: HitResultPriority,
    hitresult_generator: Option<fn(InspectTaikoPerformance<'_>) -> TaikoHitResults>,
}

impl<'map> TaikoPerformance<'map> {
    /// Create a new performance calculator for osu!taiko maps.
    ///
    /// The argument `map_or_attrs` must be either
    /// - previously calculated attributes ([`TaikoDifficultyAttributes`]
    ///   or [`TaikoPerformanceAttributes`])
    /// - a [`Beatmap`] (by reference or value)
    ///
    /// If a map is given, difficulty attributes will need to be calculated
    /// internally which is a costly operation. Hence, passing attributes
    /// should be prefered.
    ///
    /// However, when passing previously calculated attributes, make sure they
    /// have been calculated for the same map and [`Difficulty`] settings.
    /// Otherwise, the final attributes will be incorrect.
    ///
    /// [`Beatmap`]: crate::model::beatmap::Beatmap
    /// [`TaikoDifficultyAttributes`]: crate::taiko::TaikoDifficultyAttributes
    pub fn new(map: &'map Beatmap) -> Self {
        Self {
            map_or_attrs: map.into(),
            difficulty: Difficulty::new(),
            combo: None,
            acc: None,
            n300: None,
            n100: None,
            misses: None,
            hitresult_priority: HitResultPriority::BestCase,
            hitresult_generator: None,
        }
    }

    pub fn attributes(mut self, attrs: TaikoDifficultyAttributes) -> Self {
        self.map_or_attrs = MapOrAttrs::Attrs(attrs);

        self
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
    pub fn mods(mut self, mods: impl Into<GameMods>) -> Self {
        self.difficulty = self.difficulty.mods(mods);

        self
    }

    /// Specify the max combo of the play.
    pub const fn combo(mut self, combo: u32) -> Self {
        self.combo = Some(combo);

        self
    }

    /// Specify how hitresults should be generated.
    ///
    /// Defauls to [`HitResultPriority::BestCase`].
    pub const fn hitresult_priority(mut self, priority: HitResultPriority) -> Self {
        self.hitresult_priority = priority;

        self
    }

    /// Specify the amount of 300s of a play.
    pub const fn n300(mut self, n300: u32) -> Self {
        self.n300 = Some(n300);

        self
    }

    /// Specify the amount of 100s of a play.
    pub const fn n100(mut self, n100: u32) -> Self {
        self.n100 = Some(n100);

        self
    }

    /// Specify the amount of misses of the play.
    pub const fn misses(mut self, n_misses: u32) -> Self {
        self.misses = Some(n_misses);

        self
    }

    /// Specify the accuracy of a play between `0.0` and `100.0`.
    /// This will be used to generate matching hitresults.
    pub fn accuracy(mut self, acc: f64) -> Self {
        self.acc = Some(acc.clamp(0.0, 100.0) / 100.0);

        self
    }

    /// Specify how hitresults should be generated.
    pub fn hitresult_generator<H: HitResultGenerator<Taiko25>>(self) -> Self {
        Self {
            map_or_attrs: self.map_or_attrs,
            difficulty: self.difficulty,
            combo: self.combo,
            acc: self.acc,
            n300: self.n300,
            n100: self.n100,
            misses: self.misses,
            hitresult_priority: self.hitresult_priority,
            hitresult_generator: Some(H::generate_hitresults),
        }
    }

    /// Use the specified settings of the given [`Difficulty`].
    pub fn difficulty(mut self, difficulty: Difficulty) -> Self {
        self.difficulty = difficulty;

        self
    }

    /// Amount of passed objects for partial plays, e.g. a fail.
    ///
    /// If you want to calculate the performance after every few objects,
    /// instead of using [`TaikoPerformance`] multiple times with different
    /// `passed_objects`, you should use [`TaikoGradualPerformance`].
    ///
    /// [`TaikoGradualPerformance`]: crate::taiko::TaikoGradualPerformance
    pub fn passed_objects(mut self, passed_objects: u32) -> Self {
        self.difficulty = self.difficulty.passed_objects(passed_objects);

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
    pub fn clock_rate(mut self, clock_rate: f64) -> Self {
        self.difficulty = self.difficulty.clock_rate(clock_rate);

        self
    }

    /// Override a beatmap's set HP.
    ///
    /// `with_mods` determines if the given value should be used before
    /// or after accounting for mods, e.g. on `true` the value will be
    /// used as is and on `false` it will be modified based on the mods.
    ///
    /// | Minimum | Maximum |
    /// | :-----: | :-----: |
    /// | -20     | 20      |
    pub fn hp(mut self, hp: f32, with_mods: bool) -> Self {
        self.difficulty = self.difficulty.hp(hp, with_mods);

        self
    }

    /// Override a beatmap's set OD.
    ///
    /// `with_mods` determines if the given value should be used before
    /// or after accounting for mods, e.g. on `true` the value will be
    /// used as is and on `false` it will be modified based on the mods.
    ///
    /// | Minimum | Maximum |
    /// | :-----: | :-----: |
    /// | -20     | 20      |
    pub fn od(mut self, od: f32, with_mods: bool) -> Self {
        self.difficulty = self.difficulty.od(od, with_mods);

        self
    }

    /// Provide parameters through a [`TaikoScoreState`].
    pub const fn state(mut self, state: TaikoScoreState) -> Self {
        let TaikoScoreState {
            max_combo,
            n300,
            n100,
            misses,
        } = state;

        self.combo = Some(max_combo);
        self.n300 = Some(n300);
        self.n100 = Some(n100);
        self.misses = Some(misses);

        self
    }

    /// Create the [`TaikoScoreState`] that will be used for performance calculation.
    pub fn generate_state(&mut self) -> Result<TaikoScoreState, ConvertError> {
        self.map_or_attrs
            .insert_attrs(|map| super::difficulty::difficulty(&self.difficulty, map))?;

        // SAFETY: We just calculated and inserted the attributes.
        let attrs = unsafe { self.map_or_attrs.get_attrs() };

        let inspect = Taiko25::inspect_performance(self, attrs);

        let max_combo = inspect.max_combo();
        let misses = inspect.misses();

        let hitresults = match self.hitresult_generator {
            Some(generator) => generator(inspect),
            None => <Fast as HitResultGenerator<Taiko25>>::generate_hitresults(inspect),
        };

        let max_possible_combo = max_combo.saturating_sub(misses);

        let max_combo = self.combo.map_or(max_possible_combo, |combo| {
            cmp::min(combo, max_possible_combo)
        });

        self.combo = Some(max_combo);

        let TaikoHitResults { n300, n100, misses } = hitresults;

        self.n300 = Some(n300);
        self.n100 = Some(n100);
        self.misses = Some(misses);

        Ok(TaikoScoreState {
            max_combo,
            n300,
            n100,
            misses,
        })
    }

    /// Calculate all performance related values, including pp and stars.
    pub fn calculate(mut self) -> Result<TaikoPerformanceAttributes, ConvertError> {
        let state = self.generate_state()?;

        let attrs = match self.map_or_attrs {
            MapOrAttrs::Attrs(attrs) => attrs,
            MapOrAttrs::Map(ref map) => super::difficulty::difficulty(&self.difficulty, map)?,
        };

        Ok(TaikoPerformanceCalculator::new(attrs, &self.difficulty.get_mods(), state).calculate())
    }
}
