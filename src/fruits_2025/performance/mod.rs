use std::cmp::{self, Ordering};

use rosu_pp::{model::mode::ConvertError, Beatmap, Difficulty, GameMods};

use self::calculator::CatchPerformanceCalculator;

use crate::{
    any::difficulty::DifficultyExt, fruits_2025::CatchDifficultyAttributes,
    util::map_or_attrs::MapOrAttrs,
};

use super::{attributes::CatchPerformanceAttributes, score_state::CatchScoreState};

mod calculator;

/// Performance calculator on osu!catch maps.
#[derive(Clone, Debug, PartialEq)]
#[must_use]
pub struct CatchPerformance<'map> {
    map_or_attrs: MapOrAttrs<'map, CatchDifficultyAttributes>,
    difficulty: Difficulty,
    acc: Option<f64>,
    combo: Option<u32>,
    fruits: Option<u32>,
    droplets: Option<u32>,
    tiny_droplets: Option<u32>,
    tiny_droplet_misses: Option<u32>,
    misses: Option<u32>,
}

impl<'map> CatchPerformance<'map> {
    /// Create a new performance calculator for osu!catch maps.
    pub fn new(map: &'map Beatmap) -> Self {
        Self {
            map_or_attrs: map.into(),
            difficulty: Difficulty::new(),
            acc: None,
            combo: None,
            fruits: None,
            droplets: None,
            tiny_droplets: None,
            tiny_droplet_misses: None,
            misses: None,
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
    pub fn mods(mut self, mods: impl Into<GameMods>) -> Self {
        self.difficulty = self.difficulty.mods(mods);

        self
    }

    /// Specify the max combo of the play.
    pub const fn combo(mut self, combo: u32) -> Self {
        self.combo = Some(combo);

        self
    }

    /// Specify the amount of fruits of a play i.e. n300.
    pub const fn fruits(mut self, n_fruits: u32) -> Self {
        self.fruits = Some(n_fruits);

        self
    }

    /// Specify the amount of droplets of a play i.e. n100.
    pub const fn droplets(mut self, n_droplets: u32) -> Self {
        self.droplets = Some(n_droplets);

        self
    }

    /// Specify the amount of tiny droplets of a play i.e. n50.
    pub const fn tiny_droplets(mut self, n_tiny_droplets: u32) -> Self {
        self.tiny_droplets = Some(n_tiny_droplets);

        self
    }

    /// Specify the amount of tiny droplet misses of a play i.e. `n_katu`.
    pub const fn tiny_droplet_misses(mut self, n_tiny_droplet_misses: u32) -> Self {
        self.tiny_droplet_misses = Some(n_tiny_droplet_misses);

        self
    }

    /// Specify the amount of fruit / droplet misses of the play.
    pub const fn misses(mut self, n_misses: u32) -> Self {
        self.misses = Some(n_misses);

        self
    }

    /// Use the specified settings of the given [`Difficulty`].
    pub fn difficulty(mut self, difficulty: Difficulty) -> Self {
        self.difficulty = difficulty;

        self
    }

    /// Amount of passed objects for partial plays, e.g. a fail.
    ///
    /// If you want to calculate the performance after every few objects,
    /// instead of using [`CatchPerformance`] multiple times with different
    /// `passed_objects`, you should use [`CatchGradualPerformance`].
    ///
    /// [`CatchGradualPerformance`]: crate::catch::CatchGradualPerformance
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

    /// Override a beatmap's set AR.
    ///
    /// `with_mods` determines if the given value should be used before
    /// or after accounting for mods, e.g. on `true` the value will be
    /// used as is and on `false` it will be modified based on the mods.
    ///
    /// | Minimum | Maximum |
    /// | :-----: | :-----: |
    /// | -20     | 20      |
    pub fn ar(mut self, ar: f32, with_mods: bool) -> Self {
        self.difficulty = self.difficulty.ar(ar, with_mods);

        self
    }

    /// Override a beatmap's set CS.
    ///
    /// `with_mods` determines if the given value should be used before
    /// or after accounting for mods, e.g. on `true` the value will be
    /// used as is and on `false` it will be modified based on the mods.
    ///
    /// | Minimum | Maximum |
    /// | :-----: | :-----: |
    /// | -20     | 20      |
    pub fn cs(mut self, cs: f32, with_mods: bool) -> Self {
        self.difficulty = self.difficulty.cs(cs, with_mods);

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

    /// Adjust patterns as if the HR mod is enabled.
    pub fn hardrock_offsets(mut self, hardrock_offsets: bool) -> Self {
        self.difficulty = self.difficulty.hardrock_offsets(hardrock_offsets);

        self
    }

    /// Provide parameters through an [`CatchScoreState`].
    #[allow(clippy::needless_pass_by_value)]
    pub const fn state(mut self, state: CatchScoreState) -> Self {
        let CatchScoreState {
            max_combo,
            fruits: n_fruits,
            droplets: n_droplets,
            tiny_droplets: n_tiny_droplets,
            tiny_droplet_misses: n_tiny_droplet_misses,
            misses,
        } = state;

        self.combo = Some(max_combo);
        self.fruits = Some(n_fruits);
        self.droplets = Some(n_droplets);
        self.tiny_droplets = Some(n_tiny_droplets);
        self.tiny_droplet_misses = Some(n_tiny_droplet_misses);
        self.misses = Some(misses);

        self
    }

    /// Specify the accuracy of a play between `0.0` and `100.0`.
    /// This will be used to generate matching hitresults.
    pub fn accuracy(mut self, acc: f64) -> Self {
        self.acc = Some(acc.clamp(0.0, 100.0) / 100.0);

        self
    }

    /// Create the [`CatchScoreState`] that will be used for performance calculation.
    #[allow(clippy::too_many_lines)]
    pub fn generate_state(&mut self) -> Result<CatchScoreState, ConvertError> {
        self.map_or_attrs
            .insert_attrs(|map| super::difficulty::difficulty(&self.difficulty, map))?;

        // SAFETY: We just calculated and inserted the attributes.
        let attrs = unsafe { self.map_or_attrs.get_attrs() };

        let misses = self
            .misses
            .map_or(0, |n| cmp::min(n, attrs.n_fruits + attrs.n_droplets));

        let max_combo = self.combo.unwrap_or_else(|| attrs.max_combo() - misses);

        let mut best_state = CatchScoreState {
            max_combo,
            misses,
            ..Default::default()
        };

        let mut best_dist = f64::INFINITY;

        let (n_fruits, n_droplets) = match (self.fruits, self.droplets) {
            (Some(mut n_fruits), Some(mut n_droplets)) => {
                let n_remaining = (attrs.n_fruits + attrs.n_droplets)
                    .saturating_sub(n_fruits + n_droplets + misses);

                let new_droplets =
                    cmp::min(n_remaining, attrs.n_droplets.saturating_sub(n_droplets));
                n_droplets += new_droplets;
                n_fruits += n_remaining - new_droplets;

                n_fruits = cmp::min(
                    n_fruits,
                    (attrs.n_fruits + attrs.n_droplets).saturating_sub(n_droplets + misses),
                );
                n_droplets = cmp::min(
                    n_droplets,
                    attrs.n_fruits + attrs.n_droplets - n_fruits - misses,
                );

                (n_fruits, n_droplets)
            }
            (Some(mut n_fruits), None) => {
                let n_droplets = attrs
                    .n_droplets
                    .saturating_sub(misses.saturating_sub(attrs.n_fruits.saturating_sub(n_fruits)));

                n_fruits = attrs.n_fruits + attrs.n_droplets - misses - n_droplets;

                (n_fruits, n_droplets)
            }
            (None, Some(mut n_droplets)) => {
                let n_fruits = attrs.n_fruits.saturating_sub(
                    misses.saturating_sub(attrs.n_droplets.saturating_sub(n_droplets)),
                );

                n_droplets = attrs.n_fruits + attrs.n_droplets - misses - n_fruits;

                (n_fruits, n_droplets)
            }
            (None, None) => {
                let n_droplets = attrs.n_droplets.saturating_sub(misses);
                let n_fruits =
                    attrs.n_fruits - (misses - (attrs.n_droplets.saturating_sub(n_droplets)));

                (n_fruits, n_droplets)
            }
        };

        best_state.fruits = n_fruits;
        best_state.droplets = n_droplets;

        let mut find_best_tiny_droplets = |acc: f64| {
            let raw_tiny_droplets = acc
                * f64::from(attrs.n_fruits + attrs.n_droplets + attrs.n_tiny_droplets)
                - f64::from(n_fruits + n_droplets);
            let min_tiny_droplets =
                cmp::min(attrs.n_tiny_droplets, raw_tiny_droplets.floor() as u32);
            let max_tiny_droplets =
                cmp::min(attrs.n_tiny_droplets, raw_tiny_droplets.ceil() as u32);

            // Hopefully using `HitResultPriority::Fastest` wouldn't make a big
            // difference here so let's be lazy and ignore it
            for n_tiny_droplets in min_tiny_droplets..=max_tiny_droplets {
                let n_tiny_droplet_misses = attrs.n_tiny_droplets - n_tiny_droplets;

                let curr_acc = accuracy(
                    n_fruits,
                    n_droplets,
                    n_tiny_droplets,
                    n_tiny_droplet_misses,
                    misses,
                );
                let curr_dist = (acc - curr_acc).abs();

                if curr_dist < best_dist {
                    best_dist = curr_dist;
                    best_state.tiny_droplets = n_tiny_droplets;
                    best_state.tiny_droplet_misses = n_tiny_droplet_misses;
                }
            }
        };

        #[allow(clippy::single_match_else)]
        match (self.tiny_droplets, self.tiny_droplet_misses) {
            (Some(n_tiny_droplets), Some(n_tiny_droplet_misses)) => match self.acc {
                Some(acc) => {
                    match (n_tiny_droplets + n_tiny_droplet_misses).cmp(&attrs.n_tiny_droplets) {
                        Ordering::Equal => {
                            best_state.tiny_droplets = n_tiny_droplets;
                            best_state.tiny_droplet_misses = n_tiny_droplet_misses;
                        }
                        Ordering::Less | Ordering::Greater => find_best_tiny_droplets(acc),
                    }
                }
                None => {
                    let n_remaining = attrs
                        .n_tiny_droplets
                        .saturating_sub(n_tiny_droplets + n_tiny_droplet_misses);

                    best_state.tiny_droplets = n_tiny_droplets + n_remaining;
                    best_state.tiny_droplet_misses = n_tiny_droplet_misses;
                }
            },
            (Some(n_tiny_droplets), None) => {
                best_state.tiny_droplets = cmp::min(attrs.n_tiny_droplets, n_tiny_droplets);
                best_state.tiny_droplet_misses =
                    attrs.n_tiny_droplets.saturating_sub(n_tiny_droplets);
            }
            (None, Some(n_tiny_droplet_misses)) => {
                best_state.tiny_droplets =
                    attrs.n_tiny_droplets.saturating_sub(n_tiny_droplet_misses);
                best_state.tiny_droplet_misses =
                    cmp::min(attrs.n_tiny_droplets, n_tiny_droplet_misses);
            }
            (None, None) => match self.acc {
                Some(acc) => find_best_tiny_droplets(acc),
                None => best_state.tiny_droplets = attrs.n_tiny_droplets,
            },
        }

        self.combo = Some(best_state.max_combo);
        self.fruits = Some(best_state.fruits);
        self.droplets = Some(best_state.droplets);
        self.tiny_droplets = Some(best_state.tiny_droplets);
        self.tiny_droplet_misses = Some(best_state.tiny_droplet_misses);
        self.misses = Some(best_state.misses);

        Ok(best_state)
    }

    /// Calculate all performance related values, including pp and stars.
    pub fn calculate(mut self) -> Result<CatchPerformanceAttributes, ConvertError> {
        let state = self.generate_state()?;

        let attrs = match self.map_or_attrs {
            MapOrAttrs::Attrs(attrs) => attrs,
            MapOrAttrs::Map(ref map) => super::difficulty::difficulty(&self.difficulty, map)?,
        };

        Ok(CatchPerformanceCalculator::new(attrs, &self.difficulty.get_mods(), state).calculate())
    }
}

fn accuracy(
    n_fruits: u32,
    n_droplets: u32,
    n_tiny_droplets: u32,
    n_tiny_droplet_misses: u32,
    misses: u32,
) -> f64 {
    let numerator = n_fruits + n_droplets + n_tiny_droplets;
    let denominator = numerator + n_tiny_droplet_misses + misses;

    f64::from(numerator) / f64::from(denominator)
}
