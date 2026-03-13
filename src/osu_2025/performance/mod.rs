use std::cmp;

use rosu_pp::{any::HitResultPriority, model::mode::ConvertError, Beatmap, Difficulty, GameMods};

use self::calculator::OsuPerformanceCalculator;
pub use self::calculator::PERFORMANCE_BASE_MULTIPLIER;

use crate::{
    any::difficulty::DifficultyExt,
    util::{map_or_attrs::MapOrAttrs, mods::GameModsExt},
};

use super::{
    attributes::{OsuDifficultyAttributes, OsuPerformanceAttributes},
    score_state::{OsuScoreOrigin, OsuScoreState},
};

mod calculator;

/// Performance calculator on osu!standard maps.
#[derive(Clone, Debug, PartialEq)]
#[must_use]
pub struct OsuPerformance<'map> {
    pub(crate) map_or_attrs: MapOrAttrs<'map, OsuDifficultyAttributes>,
    pub(crate) difficulty: Difficulty,
    pub(crate) acc: Option<f64>,
    pub(crate) combo: Option<u32>,
    pub(crate) large_tick_hits: Option<u32>,
    pub(crate) small_tick_hits: Option<u32>,
    pub(crate) slider_end_hits: Option<u32>,
    pub(crate) n300: Option<u32>,
    pub(crate) n100: Option<u32>,
    pub(crate) n50: Option<u32>,
    pub(crate) misses: Option<u32>,
    pub(crate) hitresult_priority: HitResultPriority,
}

impl<'map> OsuPerformance<'map> {
    /// Create a new performance calculator for osu! maps.
    ///
    /// The argument `map_or_attrs` must be either
    /// - previously calculated attributes ([`OsuDifficultyAttributes`]
    ///   or [`OsuPerformanceAttributes`])
    /// - a [`Beatmap`] (by reference or value)
    ///
    /// If a map is given, difficulty attributes will need to be calculated
    /// internally which is a costly operation. Hence, passing attributes
    /// should be prefered.
    ///
    /// However, when passing previously calculated attributes, make sure they
    /// have been calculated for the same map and [`Difficulty`] settings.
    /// Otherwise, the final attributes will be incorrect.
    pub fn new(map: &'map Beatmap) -> Self {
        Self {
            map_or_attrs: map.into(),
            difficulty: Difficulty::new(),
            acc: None,
            combo: None,
            large_tick_hits: None,
            small_tick_hits: None,
            slider_end_hits: None,
            n300: None,
            n100: None,
            n50: None,
            misses: None,
            hitresult_priority: HitResultPriority::BestCase,
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

    /// Specify how hitresults should be generated.
    ///
    /// Defauls to [`HitResultPriority::BestCase`].
    pub const fn hitresult_priority(mut self, priority: HitResultPriority) -> Self {
        self.hitresult_priority = priority;

        self
    }

    /// Whether the calculated attributes belong to an osu!lazer or osu!stable
    /// score.
    ///
    /// Defaults to `true`.
    ///
    /// This affects internal accuracy calculation because lazer considers
    /// slider heads for accuracy whereas stable does not.
    pub fn lazer(mut self, lazer: bool) -> Self {
        self.difficulty = self.difficulty.lazer(lazer);

        self
    }

    /// Specify the amount of "large tick" hits.
    ///
    /// The meaning depends on the kind of score:
    /// - if set on osu!stable, this value is irrelevant and can be `0`
    /// - if set on osu!lazer *with* slider accuracy, this value is the amount
    ///   of hit slider ticks and repeats
    /// - if set on osu!lazer *without* slider accuracy, this value is the
    ///   amount of hit slider heads, ticks, and repeats
    pub const fn large_tick_hits(mut self, large_tick_hits: u32) -> Self {
        self.large_tick_hits = Some(large_tick_hits);

        self
    }

    /// Specify the amount of "small tick" hits.
    ///
    /// Only relevant for osu!lazer scores without slider accuracy. In that
    /// case, this value is the amount of slider tail hits.
    pub const fn small_tick_hits(mut self, small_tick_hits: u32) -> Self {
        self.small_tick_hits = Some(small_tick_hits);

        self
    }

    /// Specify the amount of hit slider ends.
    ///
    /// Only relevant for osu!lazer scores with slider accuracy.
    pub const fn slider_end_hits(mut self, slider_end_hits: u32) -> Self {
        self.slider_end_hits = Some(slider_end_hits);

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

    /// Specify the amount of 50s of a play.
    pub const fn n50(mut self, n50: u32) -> Self {
        self.n50 = Some(n50);

        self
    }

    /// Specify the amount of misses of a play.
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
    /// instead of using [`OsuPerformance`] multiple times with different
    /// `passed_objects`, you should use [`OsuGradualPerformance`].
    ///
    /// [`OsuGradualPerformance`]: crate::osu::OsuGradualPerformance
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

    /// Provide parameters through an [`OsuScoreState`].
    #[allow(clippy::needless_pass_by_value)]
    pub const fn state(mut self, state: OsuScoreState) -> Self {
        let OsuScoreState {
            max_combo,
            large_tick_hits,
            small_tick_hits,
            slider_end_hits,
            n300,
            n100,
            n50,
            misses,
        } = state;

        self.combo = Some(max_combo);
        self.large_tick_hits = Some(large_tick_hits);
        self.small_tick_hits = Some(small_tick_hits);
        self.slider_end_hits = Some(slider_end_hits);
        self.n300 = Some(n300);
        self.n100 = Some(n100);
        self.n50 = Some(n50);
        self.misses = Some(misses);

        self
    }

    /// Specify the accuracy of a play between `0.0` and `100.0`.
    /// This will be used to generate matching hitresults.
    pub fn accuracy(mut self, acc: f64) -> Self {
        self.acc = Some(acc.clamp(0.0, 100.0) / 100.0);

        self
    }

    /// Create the [`OsuScoreState`] that will be used for performance calculation.
    #[allow(clippy::too_many_lines)]
    pub fn generate_state(&mut self) -> Result<OsuScoreState, ConvertError> {
        self.map_or_attrs
            .insert_attrs(|map| super::difficulty::difficulty(&self.difficulty, map))?;

        // SAFETY: We just calculated and inserted the attributes.
        let attrs = unsafe { self.map_or_attrs.get_attrs() };

        let max_combo = attrs.max_combo;
        let n_objects = cmp::min(
            self.difficulty.get_passed_objects() as u32,
            attrs.n_objects(),
        );
        let priority = self.hitresult_priority;

        let misses = self.misses.map_or(0, |n| cmp::min(n, n_objects));
        let n_remaining = n_objects - misses;

        let mut n300 = self.n300.map_or(0, |n| cmp::min(n, n_remaining));
        let mut n100 = self.n100.map_or(0, |n| cmp::min(n, n_remaining));
        let mut n50 = self.n50.map_or(0, |n| cmp::min(n, n_remaining));

        let lazer = self.difficulty.get_lazer();
        let using_classic_slider_acc = self.difficulty.get_mods().no_slider_head_acc(lazer);

        let (origin, slider_end_hits, large_tick_hits, small_tick_hits) =
            match (lazer, using_classic_slider_acc) {
                (false, _) => (OsuScoreOrigin::Stable, 0, 0, 0),
                (true, false) => {
                    let origin = OsuScoreOrigin::WithSliderAcc {
                        max_large_ticks: attrs.n_large_ticks,
                        max_slider_ends: attrs.n_sliders,
                    };

                    let slider_end_hits = self
                        .slider_end_hits
                        .map_or(attrs.n_sliders, |n| cmp::min(n, attrs.n_sliders));

                    let large_tick_hits = self
                        .large_tick_hits
                        .map_or(attrs.n_large_ticks, |n| cmp::min(n, attrs.n_large_ticks));

                    (origin, slider_end_hits, large_tick_hits, 0)
                }
                (true, true) => {
                    let origin = OsuScoreOrigin::WithoutSliderAcc {
                        max_large_ticks: attrs.n_sliders + attrs.n_large_ticks,
                        max_small_ticks: attrs.n_sliders,
                    };

                    let small_tick_hits = self
                        .small_tick_hits
                        .map_or(attrs.n_sliders, |n| cmp::min(n, attrs.n_sliders));

                    let large_tick_hits = self
                        .large_tick_hits
                        .map_or(attrs.n_sliders + attrs.n_large_ticks, |n| {
                            cmp::min(n, attrs.n_sliders + attrs.n_large_ticks)
                        });

                    (origin, 0, large_tick_hits, small_tick_hits)
                }
            };

        let (slider_acc_value, max_slider_acc_value) = match origin {
            OsuScoreOrigin::Stable => (0, 0),
            OsuScoreOrigin::WithSliderAcc {
                max_large_ticks,
                max_slider_ends,
            } => (
                150 * slider_end_hits + 30 * large_tick_hits,
                150 * max_slider_ends + 30 * max_large_ticks,
            ),
            OsuScoreOrigin::WithoutSliderAcc {
                max_large_ticks,
                max_small_ticks,
            } => (
                30 * large_tick_hits + 10 * small_tick_hits,
                30 * max_large_ticks + 10 * max_small_ticks,
            ),
        };

        if let Some(acc) = self.acc {
            let target_total = acc * f64::from(300 * n_objects + max_slider_acc_value);

            match (self.n300, self.n100, self.n50) {
                (Some(_), Some(_), Some(_)) => {
                    let remaining = n_objects.saturating_sub(n300 + n100 + n50 + misses);

                    match priority {
                        HitResultPriority::BestCase => n300 += remaining,
                        HitResultPriority::WorstCase => n50 += remaining,
                    }
                }
                (Some(_), Some(_), None) => n50 = n_objects.saturating_sub(n300 + n100 + misses),
                (Some(_), None, Some(_)) => n100 = n_objects.saturating_sub(n300 + n50 + misses),
                (None, Some(_), Some(_)) => n300 = n_objects.saturating_sub(n100 + n50 + misses),
                (Some(_), None, None) => {
                    let mut best_dist = f64::MAX;

                    n300 = cmp::min(n300, n_remaining);
                    let n_remaining = n_remaining - n300;

                    let raw_n100 = (target_total
                        - f64::from(50 * n_remaining + 300 * n300 + slider_acc_value))
                        / 50.0;
                    let min_n100 = cmp::min(n_remaining, raw_n100.floor() as u32);
                    let max_n100 = cmp::min(n_remaining, raw_n100.ceil() as u32);

                    for new100 in min_n100..=max_n100 {
                        let new50 = n_remaining - new100;

                        let state = NoComboState {
                            n300,
                            n100: new100,
                            n50: new50,
                            misses,
                            large_tick_hits,
                            small_tick_hits,
                            slider_end_hits,
                        };

                        let dist = (acc - state.accuracy(origin)).abs();

                        if dist < best_dist {
                            best_dist = dist;
                            n100 = new100;
                            n50 = new50;
                        }
                    }
                }
                (None, Some(_), None) => {
                    let mut best_dist = f64::MAX;

                    n100 = cmp::min(n100, n_remaining);
                    let n_remaining = n_remaining - n100;

                    let raw_n300 = (target_total
                        - f64::from(50 * n_remaining + 100 * n100 + slider_acc_value))
                        / 250.0;
                    let min_n300 = cmp::min(n_remaining, raw_n300.floor() as u32);
                    let max_n300 = cmp::min(n_remaining, raw_n300.ceil() as u32);

                    for new300 in min_n300..=max_n300 {
                        let new50 = n_remaining - new300;

                        let state = NoComboState {
                            n300: new300,
                            n100,
                            n50: new50,
                            misses,
                            large_tick_hits,
                            small_tick_hits,
                            slider_end_hits,
                        };

                        let curr_dist = (acc - state.accuracy(origin)).abs();

                        if curr_dist < best_dist {
                            best_dist = curr_dist;
                            n300 = new300;
                            n50 = new50;
                        }
                    }
                }
                (None, None, Some(_)) => {
                    let mut best_dist = f64::MAX;

                    n50 = cmp::min(n50, n_remaining);
                    let n_remaining = n_remaining - n50;

                    let raw_n300 = (target_total + f64::from(100 * misses + 50 * n50)
                        - f64::from(100 * n_objects + slider_acc_value))
                        / 200.0;

                    let min_n300 = cmp::min(n_remaining, raw_n300.floor() as u32);
                    let max_n300 = cmp::min(n_remaining, raw_n300.ceil() as u32);

                    for new300 in min_n300..=max_n300 {
                        let new100 = n_remaining - new300;

                        let state = NoComboState {
                            n300: new300,
                            n100: new100,
                            n50,
                            misses,
                            large_tick_hits,
                            small_tick_hits,
                            slider_end_hits,
                        };

                        let curr_dist = (acc - state.accuracy(origin)).abs();

                        if curr_dist < best_dist {
                            best_dist = curr_dist;
                            n300 = new300;
                            n100 = new100;
                        }
                    }
                }
                (None, None, None) => {
                    let mut best_dist = f64::MAX;

                    let raw_n300 =
                        (target_total - f64::from(50 * n_remaining + slider_acc_value)) / 250.0;
                    let min_n300 = cmp::min(n_remaining, raw_n300.floor() as u32);
                    let max_n300 = cmp::min(n_remaining, raw_n300.ceil() as u32);

                    for new300 in min_n300..=max_n300 {
                        let raw_n100 = (target_total
                            - f64::from(50 * n_remaining + 250 * new300 + slider_acc_value))
                            / 50.0;
                        let min_n100 = cmp::min(raw_n100.floor() as u32, n_remaining - new300);
                        let max_n100 = cmp::min(raw_n100.ceil() as u32, n_remaining - new300);

                        for new100 in min_n100..=max_n100 {
                            let new50 = n_remaining - new300 - new100;

                            let state = NoComboState {
                                n300: new300,
                                n100: new100,
                                n50: new50,
                                misses,
                                large_tick_hits,
                                small_tick_hits,
                                slider_end_hits,
                            };

                            let curr_dist = (acc - state.accuracy(origin)).abs();

                            if curr_dist < best_dist {
                                best_dist = curr_dist;
                                n300 = new300;
                                n100 = new100;
                                n50 = new50;
                            }
                        }
                    }

                    match priority {
                        HitResultPriority::BestCase => {
                            // Shift n50 to n100 by sacrificing n300
                            let n = cmp::min(n300, n50 / 4);
                            n300 -= n;
                            n100 += 5 * n;
                            n50 -= 4 * n;
                        }
                        HitResultPriority::WorstCase => {
                            // Shift n100 to n50 by gaining n300
                            let n = n100 / 5;
                            n300 += n;
                            n100 -= 5 * n;
                            n50 += 4 * n;
                        }
                    }
                }
            }
        } else {
            let remaining = n_objects.saturating_sub(n300 + n100 + n50 + misses);

            match priority {
                HitResultPriority::BestCase => match (self.n300, self.n100, self.n50) {
                    (None, ..) => n300 = remaining,
                    (_, None, _) => n100 = remaining,
                    (.., None) => n50 = remaining,
                    _ => n300 += remaining,
                },
                HitResultPriority::WorstCase => match (self.n50, self.n100, self.n300) {
                    (None, ..) => n50 = remaining,
                    (_, None, _) => n100 = remaining,
                    (.., None) => n300 = remaining,
                    _ => n50 += remaining,
                },
            }
        }

        let max_possible_combo = max_combo.saturating_sub(misses);

        let max_combo = self.combo.map_or(max_possible_combo, |combo| {
            cmp::min(combo, max_possible_combo)
        });

        self.combo = Some(max_combo);
        self.slider_end_hits = Some(slider_end_hits);
        self.large_tick_hits = Some(large_tick_hits);
        self.small_tick_hits = Some(small_tick_hits);
        self.n300 = Some(n300);
        self.n100 = Some(n100);
        self.n50 = Some(n50);
        self.misses = Some(misses);

        Ok(OsuScoreState {
            max_combo,
            large_tick_hits,
            small_tick_hits,
            slider_end_hits,
            n300,
            n100,
            n50,
            misses,
        })
    }

    /// Calculate all performance related values, including pp and stars.
    pub fn calculate(mut self) -> Result<OsuPerformanceAttributes, ConvertError> {
        let state = self.generate_state()?;

        let attrs = match self.map_or_attrs {
            MapOrAttrs::Attrs(attrs) => attrs,
            MapOrAttrs::Map(ref map) => super::difficulty::difficulty(&self.difficulty, map)?,
        };

        let mods = self.difficulty.get_mods();
        let lazer = self.difficulty.get_lazer();
        let using_classic_slider_acc = mods.no_slider_head_acc(lazer);

        let mut effective_miss_count = f64::from(state.misses);

        if attrs.n_sliders > 0 {
            if using_classic_slider_acc {
                // * Consider that full combo is maximum combo minus dropped slider tails since they don't contribute to combo but also don't break it
                // * In classic scores we can't know the amount of dropped sliders so we estimate to 10% of all sliders on the map
                let full_combo_threshold =
                    f64::from(attrs.max_combo) - 0.1 * f64::from(attrs.n_sliders);

                if f64::from(state.max_combo) < full_combo_threshold {
                    effective_miss_count =
                        full_combo_threshold / f64::from(state.max_combo).max(1.0);
                }

                // * In classic scores there can't be more misses than a sum of all non-perfect judgements
                effective_miss_count = effective_miss_count.min(total_imperfect_hits(&state));
            } else {
                let full_combo_threshold =
                    f64::from(attrs.max_combo - n_slider_ends_dropped(&attrs, &state));

                if f64::from(state.max_combo) < full_combo_threshold {
                    effective_miss_count =
                        full_combo_threshold / f64::from(state.max_combo).max(1.0);
                }

                // * Combine regular misses with tick misses since tick misses break combo as well
                effective_miss_count = effective_miss_count
                    .min(f64::from(n_large_tick_miss(&attrs, &state) + state.misses));
            }
        }

        effective_miss_count = effective_miss_count.max(f64::from(state.misses));
        effective_miss_count = effective_miss_count.min(f64::from(state.total_hits()));

        let origin = match (lazer, using_classic_slider_acc) {
            (false, _) => OsuScoreOrigin::Stable,
            (true, false) => OsuScoreOrigin::WithSliderAcc {
                max_large_ticks: attrs.n_large_ticks,
                max_slider_ends: attrs.n_sliders,
            },
            (true, true) => OsuScoreOrigin::WithoutSliderAcc {
                max_large_ticks: attrs.n_sliders + attrs.n_large_ticks,
                max_small_ticks: attrs.n_sliders,
            },
        };

        let acc = state.accuracy(origin);

        let inner = OsuPerformanceCalculator::new(
            attrs,
            &mods,
            acc,
            state,
            effective_miss_count,
            using_classic_slider_acc,
        );

        Ok(inner.calculate())
    }
}

fn total_imperfect_hits(state: &OsuScoreState) -> f64 {
    f64::from(state.n100 + state.n50 + state.misses)
}

const fn n_slider_ends_dropped(attrs: &OsuDifficultyAttributes, state: &OsuScoreState) -> u32 {
    attrs.n_sliders - state.slider_end_hits
}

const fn n_large_tick_miss(attrs: &OsuDifficultyAttributes, state: &OsuScoreState) -> u32 {
    attrs.n_large_ticks - state.large_tick_hits
}

struct NoComboState {
    n300: u32,
    n100: u32,
    n50: u32,
    misses: u32,
    large_tick_hits: u32,
    small_tick_hits: u32,
    slider_end_hits: u32,
}

impl NoComboState {
    fn accuracy(&self, origin: OsuScoreOrigin) -> f64 {
        let mut numerator = 300 * self.n300 + 100 * self.n100 + 50 * self.n50;
        let mut denominator = 300 * (self.n300 + self.n100 + self.n50 + self.misses);

        match origin {
            OsuScoreOrigin::Stable => {}
            OsuScoreOrigin::WithSliderAcc {
                max_large_ticks,
                max_slider_ends,
            } => {
                let slider_end_hits = self.slider_end_hits.min(max_slider_ends);
                let large_tick_hits = self.large_tick_hits.min(max_large_ticks);

                numerator += 150 * slider_end_hits + 30 * large_tick_hits;
                denominator += 150 * max_slider_ends + 30 * max_large_ticks;
            }
            OsuScoreOrigin::WithoutSliderAcc {
                max_large_ticks,
                max_small_ticks,
            } => {
                let large_tick_hits = self.large_tick_hits.min(max_large_ticks);
                let small_tick_hits = self.small_tick_hits.min(max_small_ticks);

                numerator += 30 * large_tick_hits + 10 * small_tick_hits;
                denominator += 30 * max_large_ticks + 10 * max_small_ticks;
            }
        }

        if denominator == 0 {
            0.0
        } else {
            f64::from(numerator) / f64::from(denominator)
        }
    }
}
