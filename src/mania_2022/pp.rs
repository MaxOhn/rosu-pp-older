use std::cmp;

use rosu_pp::{any::HitResultPriority, mania::ManiaScoreState, Beatmap};

use crate::util::mods::Mods;

use super::{ManiaDifficultyAttributes, ManiaPerformanceAttributes, ManiaStars};

/// Performance calculator on osu!mania maps.
#[derive(Clone, Debug, PartialEq)]
#[must_use]
pub struct ManiaPP<'map> {
    map: &'map Beatmap,
    attributes: Option<ManiaDifficultyAttributes>,
    difficulty: ManiaStars,
    n320: Option<u32>,
    n300: Option<u32>,
    n200: Option<u32>,
    n100: Option<u32>,
    n50: Option<u32>,
    misses: Option<u32>,
    acc: Option<f64>,
    hitresult_priority: HitResultPriority,
}

impl<'map> ManiaPP<'map> {
    pub fn new(map: &'map Beatmap) -> Self {
        Self {
            map,
            attributes: None,
            difficulty: ManiaStars::new(),
            n320: None,
            n300: None,
            n200: None,
            n100: None,
            n50: None,
            misses: None,
            acc: None,
            hitresult_priority: HitResultPriority::default(),
        }
    }

    /// Provide the result of a previous difficulty or performance calculation.
    /// If you already calculated the attributes for the current map-mod combination,
    /// be sure to put them in here so that they don't have to be recalculated.
    #[inline]
    pub fn attributes(mut self, attributes: ManiaDifficultyAttributes) -> Self {
        self.attributes = Some(attributes);

        self
    }

    /// Specify mods.
    ///
    /// See <https://github.com/ppy/osu-api/wiki#mods>
    pub fn mods(mut self, mods: u32) -> Self {
        self.difficulty = self.difficulty.mods(mods);

        self
    }

    /// Use the specified settings of the given [`Difficulty`].
    pub fn difficulty(mut self, difficulty: ManiaStars) -> Self {
        self.difficulty = difficulty;

        self
    }

    /// Amount of passed objects for partial plays, e.g. a fail.
    ///
    /// If you want to calculate the performance after every few objects,
    /// instead of using [`ManiaPerformance`] multiple times with different
    /// `passed_objects`, you should use [`ManiaGradualPerformance`].
    ///
    /// [`ManiaGradualPerformance`]: crate::mania::ManiaGradualPerformance
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

    /// Specify the accuracy of a play between `0.0` and `100.0`.
    /// This will be used to generate matching hitresults.
    pub fn accuracy(mut self, acc: f64) -> Self {
        self.acc = Some(acc.clamp(0.0, 100.0) / 100.0);

        self
    }

    /// Specify how hitresults should be generated.
    ///
    /// Defauls to [`HitResultPriority::BestCase`].
    pub const fn hitresult_priority(mut self, priority: HitResultPriority) -> Self {
        self.hitresult_priority = priority;

        self
    }

    /// Specify the amount of 320s of a play.
    pub const fn n320(mut self, n320: u32) -> Self {
        self.n320 = Some(n320);

        self
    }

    /// Specify the amount of 300s of a play.
    pub const fn n300(mut self, n300: u32) -> Self {
        self.n300 = Some(n300);

        self
    }

    /// Specify the amount of 200s of a play.
    pub const fn n200(mut self, n200: u32) -> Self {
        self.n200 = Some(n200);

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

    /// Provide parameters through an [`ManiaScoreState`].
    #[allow(clippy::needless_pass_by_value)]
    pub const fn state(mut self, state: ManiaScoreState) -> Self {
        let ManiaScoreState {
            n320,
            n300,
            n200,
            n100,
            n50,
            misses,
        } = state;

        self.n320 = Some(n320);
        self.n300 = Some(n300);
        self.n200 = Some(n200);
        self.n100 = Some(n100);
        self.n50 = Some(n50);
        self.misses = Some(misses);

        self
    }

    #[allow(clippy::too_many_lines, clippy::similar_names)]
    fn generate_state(&mut self) -> (ManiaScoreState, ManiaDifficultyAttributes) {
        let attrs = self
            .attributes
            .take()
            .unwrap_or_else(|| self.difficulty.calculate(self.map));

        let n_objects = cmp::min(self.difficulty.get_passed_objects() as u32, attrs.n_objects);

        let priority = self.hitresult_priority;

        let misses = self.misses.map_or(0, |n| cmp::min(n, n_objects));
        let n_remaining = n_objects - misses;

        let mut n320 = self.n320.map_or(0, |n| cmp::min(n, n_remaining));
        let mut n300 = self.n300.map_or(0, |n| cmp::min(n, n_remaining));
        let mut n200 = self.n200.map_or(0, |n| cmp::min(n, n_remaining));
        let mut n100 = self.n100.map_or(0, |n| cmp::min(n, n_remaining));
        let mut n50 = self.n50.map_or(0, |n| cmp::min(n, n_remaining));

        if let Some(acc) = self.acc {
            let target_total = acc * f64::from(6 * n_objects);

            match (self.n320, self.n300, self.n200, self.n100, self.n50) {
                // All hitresults given
                (Some(_), Some(_), Some(_), Some(_), Some(_)) => {
                    let remaining =
                        n_objects.saturating_sub(n320 + n300 + n200 + n100 + n50 + misses);

                    match priority {
                        HitResultPriority::BestCase => n320 += remaining,
                        HitResultPriority::WorstCase => n50 += remaining,
                    }
                }

                // All but one hitresults given
                (None, Some(_), Some(_), Some(_), Some(_)) => {
                    n320 = n_objects.saturating_sub(n300 + n200 + n100 + n50 + misses);
                }
                (Some(_), None, Some(_), Some(_), Some(_)) => {
                    n300 = n_objects.saturating_sub(n320 + n200 + n100 + n50 + misses);
                }
                (Some(_), Some(_), None, Some(_), Some(_)) => {
                    n200 = n_objects.saturating_sub(n320 + n300 + n100 + n50 + misses);
                }
                (Some(_), Some(_), Some(_), None, Some(_)) => {
                    n100 = n_objects.saturating_sub(n320 + n300 + n200 + n50 + misses);
                }
                (Some(_), Some(_), Some(_), Some(_), None) => {
                    n50 = n_objects.saturating_sub(n320 + n300 + n200 + n100 + misses);
                }

                // n200, n100, and n50 given
                (None, None, Some(_), Some(_), Some(_)) => {
                    let n_remaining =
                        n_objects.saturating_sub(n320 + n300 + n200 + n100 + n50 + misses);

                    match priority {
                        HitResultPriority::BestCase => n320 = n_remaining,
                        HitResultPriority::WorstCase => n300 = n_remaining,
                    }
                }

                // n100 and n50 given
                (.., None, Some(_), Some(_)) => {
                    let mut best_dist = f64::INFINITY;
                    let mut n3x0 = n_objects.saturating_sub(n320 + n300 + n100 + n50 + misses);

                    let raw_n3x0 = (target_total - f64::from(4 * n_remaining)
                        + f64::from(2 * n100 + 3 * n50))
                        / 2.0;
                    let min_n3x0 = cmp::min(
                        raw_n3x0.floor() as u32,
                        n_remaining.saturating_sub(n100 + n50),
                    );
                    let max_n3x0 = cmp::min(
                        raw_n3x0.ceil() as u32,
                        n_remaining.saturating_sub(n100 + n50),
                    );

                    let (min_n3x0, max_n3x0) = match (self.n320, self.n300) {
                        (Some(_), Some(_)) => (n320 + n300, n320 + n300),
                        (Some(_), None) => (cmp::max(min_n3x0, n320), cmp::max(max_n3x0, n320)),
                        (None, Some(_)) => (cmp::max(min_n3x0, n300), cmp::max(max_n3x0, n300)),
                        (None, None) => (min_n3x0, max_n3x0),
                    };

                    for new3x0 in min_n3x0..=max_n3x0 {
                        let new200 = n_remaining.saturating_sub(new3x0 + n100 + n50);
                        let curr_dist =
                            (acc - accuracy(new3x0, 0, new200, n100, n50, misses)).abs();

                        if curr_dist < best_dist {
                            best_dist = curr_dist;
                            n3x0 = new3x0;
                            n200 = new200;
                        }
                    }

                    match (self.n320, self.n300) {
                        (None, None) => match priority {
                            HitResultPriority::BestCase => n320 = n3x0,
                            HitResultPriority::WorstCase => n300 = n3x0,
                        },
                        (Some(_), None) => n300 = n3x0 - n320,
                        (None, Some(_)) => n320 = n3x0 - n300,
                        _ => {}
                    }
                }

                // n200 and n50 given
                (.., Some(_), None, Some(_)) => {
                    let mut best_dist = f64::INFINITY;
                    let mut n3x0 = n_objects.saturating_sub(n320 + n300 + n200 + n50 + misses);

                    let raw_n3x0 = (target_total - f64::from(2 * (n_remaining + n200) - n50)) / 4.0;
                    let min_n3x0 = cmp::min(
                        raw_n3x0.floor() as u32,
                        n_remaining.saturating_sub(n200 + n50),
                    );
                    let max_n3x0 = cmp::min(
                        raw_n3x0.ceil() as u32,
                        n_remaining.saturating_sub(n200 + n50),
                    );

                    let (min_n3x0, max_n3x0) = match (self.n320, self.n300) {
                        (Some(_), Some(_)) => (n320 + n300, n320 + n300),
                        (Some(_), None) => (cmp::max(min_n3x0, n320), cmp::max(max_n3x0, n320)),
                        (None, Some(_)) => (cmp::max(min_n3x0, n300), cmp::max(max_n3x0, n300)),
                        (None, None) => (min_n3x0, max_n3x0),
                    };

                    for new3x0 in min_n3x0..=max_n3x0 {
                        let new100 = n_remaining.saturating_sub(new3x0 + n200 + n50);
                        let curr_dist =
                            (acc - accuracy(new3x0, 0, n200, new100, n50, misses)).abs();

                        if curr_dist < best_dist {
                            best_dist = curr_dist;
                            n3x0 = new3x0;
                            n100 = new100;
                        }
                    }

                    match (self.n320, self.n300) {
                        (None, None) => match priority {
                            HitResultPriority::BestCase => n320 = n3x0,
                            HitResultPriority::WorstCase => n300 = n3x0,
                        },
                        (Some(_), None) => n300 = n3x0 - n320,
                        (None, Some(_)) => n320 = n3x0 - n300,
                        _ => {}
                    }
                }

                // n200 and n100 given
                (.., Some(_), Some(_), None) => {
                    let mut best_dist = f64::INFINITY;
                    let mut n3x0 = n_objects.saturating_sub(n320 + n300 + n200 + n100 + misses);

                    let raw_n3x0 = (target_total - f64::from(n_remaining + 3 * n200 + n100)) / 5.0;
                    let min_n3x0 = cmp::min(
                        raw_n3x0.floor() as u32,
                        n_remaining.saturating_sub(n200 + n100),
                    );
                    let max_n3x0 = cmp::min(
                        raw_n3x0.ceil() as u32,
                        n_remaining.saturating_sub(n200 + n100),
                    );

                    let (min_n3x0, max_n3x0) = match (self.n320, self.n300) {
                        (Some(_), Some(_)) => (n320 + n300, n320 + n300),
                        (Some(_), None) => (cmp::max(min_n3x0, n320), cmp::max(max_n3x0, n320)),
                        (None, Some(_)) => (cmp::max(min_n3x0, n300), cmp::max(max_n3x0, n300)),
                        (None, None) => (min_n3x0, max_n3x0),
                    };

                    for new3x0 in min_n3x0..=max_n3x0 {
                        let new50 = n_remaining.saturating_sub(new3x0 + n200 + n100);
                        let curr_dist =
                            (acc - accuracy(new3x0, 0, n200, n100, new50, misses)).abs();

                        if curr_dist < best_dist {
                            best_dist = curr_dist;
                            n3x0 = new3x0;
                            n50 = new50;
                        }
                    }

                    match (self.n320, self.n300) {
                        (None, None) => match priority {
                            HitResultPriority::BestCase => n320 = n3x0,
                            HitResultPriority::WorstCase => n300 = n3x0,
                        },
                        (Some(_), None) => n300 = n3x0 - n320,
                        (None, Some(_)) => n320 = n3x0 - n300,
                        _ => {}
                    }
                }

                // n200 given
                (.., Some(_), None, None) => {
                    let mut best_dist = f64::INFINITY;
                    let mut n3x0 = n_objects.saturating_sub(n320 + n300 + n200 + misses);

                    let min_n3x0 = cmp::min(
                        ((target_total - f64::from(2 * (n_remaining + n200))) / 4.0).floor() as u32,
                        n_remaining - n200,
                    );

                    let max_n3x0 = cmp::min(
                        ((target_total - f64::from(n_remaining + 3 * n200)) / 5.0).ceil() as u32,
                        n_remaining - n200,
                    );

                    let (min_n3x0, max_n3x0) = match (self.n320, self.n300) {
                        (Some(_), Some(_)) => (
                            cmp::min(n_remaining, n320 + n300),
                            cmp::min(n_remaining, n320 + n300),
                        ),
                        (Some(_), None) => (cmp::max(min_n3x0, n320), cmp::max(max_n3x0, n320)),
                        (None, Some(_)) => (cmp::max(min_n3x0, n300), cmp::max(max_n3x0, n300)),
                        (None, None) => (min_n3x0, max_n3x0),
                    };

                    for new3x0 in min_n3x0..=max_n3x0 {
                        let raw_n100 =
                            target_total - f64::from(n_remaining + 5 * new3x0 + 3 * n200);
                        let min_n100 = cmp::min(
                            raw_n100.floor() as u32,
                            n_remaining.saturating_sub(new3x0 + n200),
                        );
                        let max_n100 = cmp::min(
                            raw_n100.ceil() as u32,
                            n_remaining.saturating_sub(new3x0 + n200),
                        );

                        for new100 in min_n100..=max_n100 {
                            let new50 = n_remaining.saturating_sub(new3x0 + n200 + new100);
                            let curr_dist =
                                (acc - accuracy(new3x0, 0, n200, new100, new50, misses)).abs();

                            if curr_dist < best_dist {
                                best_dist = curr_dist;
                                n3x0 = new3x0;
                                n100 = new100;
                                n50 = new50;
                            }
                        }
                    }

                    match (self.n320, self.n300) {
                        (None, None) => match priority {
                            HitResultPriority::BestCase => n320 = n3x0,
                            HitResultPriority::WorstCase => n300 = n3x0,
                        },
                        (Some(_), None) => n300 = n3x0 - n320,
                        (None, Some(_)) => n320 = n3x0 - n300,
                        _ => {}
                    }
                }

                // n100 given
                (.., None, Some(_), None) => {
                    let mut best_dist = f64::INFINITY;
                    let mut n3x0 = n_objects.saturating_sub(n320 + n300 + n100 + misses);

                    let min_n3x0 = cmp::min(
                        (acc * f64::from(3 * n_remaining) - f64::from(2 * n_remaining - n100))
                            .floor() as u32,
                        n_remaining - n100,
                    );

                    let max_n3x0 = cmp::min(
                        ((target_total - f64::from(n_remaining + n100)) / 5.0).ceil() as u32,
                        n_remaining - n100,
                    );

                    let (min_n3x0, max_n3x0) = match (self.n320, self.n300) {
                        (Some(_), Some(_)) => (
                            cmp::min(n_remaining, n320 + n300),
                            cmp::min(n_remaining, n320 + n300),
                        ),
                        (Some(_), None) => (cmp::max(min_n3x0, n320), cmp::max(max_n3x0, n320)),
                        (None, Some(_)) => (cmp::max(min_n3x0, n300), cmp::max(max_n3x0, n300)),
                        (None, None) => (min_n3x0, max_n3x0),
                    };

                    for new3x0 in min_n3x0..=max_n3x0 {
                        let raw_n200 =
                            (target_total - f64::from(n_remaining + 5 * new3x0 + n100)) / 3.0;
                        let min_n200 = cmp::min(
                            raw_n200.floor() as u32,
                            n_remaining.saturating_sub(new3x0 + n100),
                        );
                        let max_n200 = cmp::min(
                            raw_n200.ceil() as u32,
                            n_remaining.saturating_sub(new3x0 + n100),
                        );

                        for new200 in min_n200..=max_n200 {
                            let new50 = n_remaining.saturating_sub(new3x0 + new200 + n100);
                            let curr_dist =
                                (acc - accuracy(new3x0, 0, new200, n100, new50, misses)).abs();

                            if curr_dist < best_dist {
                                best_dist = curr_dist;
                                n3x0 = new3x0;
                                n200 = new200;
                                n50 = new50;
                            }
                        }
                    }

                    match (self.n320, self.n300) {
                        (None, None) => match priority {
                            HitResultPriority::BestCase => n320 = n3x0,
                            HitResultPriority::WorstCase => n300 = n3x0,
                        },
                        (Some(_), None) => n300 = n3x0 - n320,
                        (None, Some(_)) => n320 = n3x0 - n300,
                        _ => {}
                    }
                }

                // n50 given
                (.., None, None, Some(_)) => {
                    let mut best_dist = f64::INFINITY;
                    let mut n3x0 = n_objects.saturating_sub(n320 + n300 + n50 + misses);

                    let min_n3x0 = cmp::min(
                        ((target_total - f64::from(4 * n_remaining - 3 * n50)) / 2.0).floor()
                            as u32,
                        n_remaining - n50,
                    );

                    let max_n3x0 = cmp::min(
                        ((target_total - f64::from(2 * n_remaining - n50)) / 4.0).ceil() as u32,
                        n_remaining - n50,
                    );

                    let (min_n3x0, max_n3x0) = match (self.n320, self.n300) {
                        (Some(_), Some(_)) => (
                            cmp::min(n_remaining, n320 + n300),
                            cmp::min(n_remaining, n320 + n300),
                        ),
                        (Some(_), None) => (cmp::max(min_n3x0, n320), cmp::max(max_n3x0, n320)),
                        (None, Some(_)) => (cmp::max(min_n3x0, n300), cmp::max(max_n3x0, n300)),
                        (None, None) => (min_n3x0, max_n3x0),
                    };

                    for new3x0 in min_n3x0..=max_n3x0 {
                        let raw_n200 = (target_total - f64::from(2 * n_remaining + 4 * new3x0)
                            + f64::from(n50))
                            / 2.0;
                        let min_n200 = cmp::min(
                            raw_n200.floor() as u32,
                            n_remaining.saturating_sub(new3x0 + n50),
                        );
                        let max_n200 = cmp::min(
                            raw_n200.ceil() as u32,
                            n_remaining.saturating_sub(new3x0 + n50),
                        );

                        for new200 in min_n200..=max_n200 {
                            let new100 = n_remaining.saturating_sub(new3x0 + new200 + n50);
                            let curr_dist =
                                (acc - accuracy(new3x0, 0, new200, new100, n50, misses)).abs();

                            if curr_dist < best_dist {
                                best_dist = curr_dist;
                                n3x0 = new3x0;
                                n200 = new200;
                                n100 = new100;
                            }
                        }
                    }

                    match (self.n320, self.n300) {
                        (None, None) => match priority {
                            HitResultPriority::BestCase => n320 = n3x0,
                            HitResultPriority::WorstCase => n300 = n3x0,
                        },
                        (Some(_), None) => n300 = n3x0 - n320,
                        (None, Some(_)) => n320 = n3x0 - n300,
                        _ => {}
                    }

                    if self.n320.is_none() {
                        if let HitResultPriority::BestCase = priority {
                            // Distribute n200 onto n320 and n100
                            let n = n200 / 2;
                            n320 += n;
                            n200 -= 2 * n;
                            n100 += n;
                        }
                    }
                }

                // Neither n200, n100, nor n50 given
                (.., None, None, None) => {
                    let mut best_dist = f64::INFINITY;
                    let mut n3x0 = n_objects.saturating_sub(n320 + n300 + n200 + n100 + misses);

                    let min_n3x0 = cmp::min(
                        ((target_total - f64::from(4 * n_remaining)) / 5.0).floor() as u32,
                        n_remaining,
                    );

                    let max_n3x0 = cmp::min(
                        ((target_total - f64::from(n_remaining)) / 5.0)
                            .min(acc * f64::from(3 * n_objects) - f64::from(n_remaining))
                            .ceil() as u32,
                        n_remaining,
                    );

                    let (min_n3x0, max_n3x0) = match (self.n320, self.n300) {
                        (Some(_), Some(_)) => (
                            cmp::min(n_remaining, n320 + n300),
                            cmp::min(n_remaining, n320 + n300),
                        ),
                        (Some(_), None) => (cmp::max(min_n3x0, n320), cmp::max(max_n3x0, n320)),
                        (None, Some(_)) => (cmp::max(min_n3x0, n300), cmp::max(max_n3x0, n300)),
                        (None, None) => (min_n3x0, max_n3x0),
                    };

                    for new3x0 in min_n3x0..=max_n3x0 {
                        let min_n200 = cmp::min(
                            (acc * f64::from(3 * n_objects) - f64::from(n_remaining + 2 * new3x0))
                                .floor() as u32,
                            n_remaining - new3x0,
                        );

                        let max_n200 = cmp::min(
                            ((target_total - f64::from(n_remaining + 5 * new3x0)) / 3.0).ceil()
                                as u32,
                            n_remaining - new3x0,
                        );

                        for new200 in min_n200..=max_n200 {
                            let raw_n100 =
                                target_total - f64::from(n_remaining + 5 * new3x0 + 3 * new200);
                            let min_n100 =
                                cmp::min(raw_n100.floor() as u32, n_remaining - (new3x0 + new200));
                            let max_n100 =
                                cmp::min(raw_n100.ceil() as u32, n_remaining - (new3x0 + new200));

                            for new100 in min_n100..=max_n100 {
                                let new50 = n_remaining - new3x0 - new200 - new100;
                                let curr_acc = accuracy(new3x0, 0, new200, new100, new50, misses);
                                let curr_dist = (acc - curr_acc).abs();

                                if curr_dist < best_dist {
                                    best_dist = curr_dist;
                                    n3x0 = new3x0;
                                    n200 = new200;
                                    n100 = new100;
                                    n50 = new50;
                                }
                            }
                        }
                    }

                    match (self.n320, self.n300) {
                        (None, None) => match priority {
                            HitResultPriority::BestCase => n320 = n3x0,
                            HitResultPriority::WorstCase => n300 = n3x0,
                        },
                        (Some(_), None) => n300 = n3x0 - n320,
                        (None, Some(_)) => n320 = n3x0 - n300,
                        _ => {}
                    }

                    if self.n320.is_none() {
                        if let HitResultPriority::BestCase = priority {
                            // Distribute n200 onto n320 and n100
                            let n = n200 / 2;
                            n320 += n;
                            n200 -= 2 * n;
                            n100 += n;
                        }
                    }
                }
            }
        } else {
            let remaining = n_objects.saturating_sub(n320 + n300 + n200 + n100 + n50 + misses);

            match priority {
                HitResultPriority::BestCase => {
                    match (self.n320, self.n300, self.n200, self.n100, self.n50) {
                        (None, ..) => n320 = remaining,
                        (_, None, ..) => n300 = remaining,
                        (_, _, None, ..) => n200 = remaining,
                        (.., None, _) => n100 = remaining,
                        (.., None) => n50 = remaining,
                        _ => n320 += remaining,
                    }
                }
                HitResultPriority::WorstCase => {
                    match (self.n50, self.n100, self.n200, self.n300, self.n320) {
                        (None, ..) => n50 = remaining,
                        (_, None, ..) => n100 = remaining,
                        (_, _, None, ..) => n200 = remaining,
                        (.., None, _) => n300 = remaining,
                        (.., None) => n320 = remaining,
                        _ => n50 += remaining,
                    }
                }
            }
        }

        let state = ManiaScoreState {
            n320,
            n300,
            n200,
            n100,
            n50,
            misses,
        };

        (state, attrs)
    }

    /// Calculate all performance related values, including pp and stars.
    pub fn calculate(mut self) -> ManiaPerformanceAttributes {
        let (state, attrs) = self.generate_state();

        let inner = ManiaPerformanceInner {
            mods: self.difficulty.get_mods(),
            attrs,
            state,
        };

        inner.calculate()
    }
}

struct ManiaPerformanceInner {
    attrs: ManiaDifficultyAttributes,
    mods: u32,
    state: ManiaScoreState,
}

impl ManiaPerformanceInner {
    fn calculate(self) -> ManiaPerformanceAttributes {
        // * Arbitrary initial value for scaling pp in order to standardize distributions across game modes.
        // * The specific number has no intrinsic meaning and can be adjusted as needed.
        let mut multiplier = 8.0;

        if self.mods.nf() {
            multiplier *= 0.75;
        }

        if self.mods.ez() {
            multiplier *= 0.5;
        }

        let difficulty_value = self.compute_difficulty_value();
        let pp = difficulty_value * multiplier;

        ManiaPerformanceAttributes {
            difficulty: self.attrs,
            pp,
            pp_difficulty: difficulty_value,
        }
    }

    fn compute_difficulty_value(&self) -> f64 {
        // * Star rating to pp curve
        (self.attrs.stars - 0.15).max(0.05).powf(2.2)
             // * From 80% accuracy, 1/20th of total pp is awarded per additional 1% accuracy
             * (5.0 * self.calculate_custom_accuracy() - 4.0).max(0.0)
             // * Length bonus, capped at 1500 notes
             * (1.0 + 0.1 * (self.total_hits() / 1500.0).min(1.0))
    }

    const fn total_hits(&self) -> f64 {
        self.state.total_hits() as f64
    }

    fn calculate_custom_accuracy(&self) -> f64 {
        let ManiaScoreState {
            n320,
            n300,
            n200,
            n100,
            n50,
            misses: _,
        } = &self.state;

        let total_hits = self.state.total_hits();

        if total_hits == 0 {
            return 0.0;
        }

        custom_accuracy(*n320, *n300, *n200, *n100, *n50, total_hits)
    }
}

fn custom_accuracy(n320: u32, n300: u32, n200: u32, n100: u32, n50: u32, total_hits: u32) -> f64 {
    let numerator = n320 * 32 + n300 * 30 + n200 * 20 + n100 * 10 + n50 * 5;
    let denominator = total_hits * 32;

    f64::from(numerator) / f64::from(denominator)
}

fn accuracy(n320: u32, n300: u32, n200: u32, n100: u32, n50: u32, misses: u32) -> f64 {
    let numerator = 6 * (n320 + n300) + 4 * n200 + 2 * n100 + n50;
    let denominator = 6 * (n320 + n300 + n200 + n100 + n50 + misses);

    f64::from(numerator) / f64::from(denominator)
}
