use rosu_pp::{Beatmap, Mods};

use super::{ManiaDifficultyAttributes, ManiaPerformanceAttributes, ManiaStars};

/// Performance calculator on osu!mania maps.
///
/// # Example
///
/// ```
/// use rosu_pp::{ManiaPP, Beatmap};
///
/// # /*
/// let map: Beatmap = ...
/// # */
/// # let map = Beatmap::default();
///
/// let pp_result = ManiaPP::new(&map)
///     .mods(64) // DT
///     .score(765_432)
///     .calculate();
///
/// println!("PP: {} | Stars: {}", pp_result.pp(), pp_result.stars());
///
/// let next_result = ManiaPP::new(&map)
///     .attributes(pp_result)  // reusing previous results for performance
///     .mods(8 + 64)           // has to be the same to reuse attributes
///     .score(950_000)
///     .calculate();
///
/// println!("PP: {} | Stars: {}", next_result.pp(), next_result.stars());
/// ```
#[derive(Clone, Debug)]
#[allow(clippy::upper_case_acronyms)]
pub struct ManiaPP<'map> {
    map: &'map Beatmap,
    stars: Option<f64>,
    mods: u32,
    pub(crate) score: Option<f64>,
    passed_objects: Option<usize>,
}

impl<'map> ManiaPP<'map> {
    /// Create a new performance calculator for osu!mania maps.
    #[inline]
    pub fn new(map: &'map Beatmap) -> Self {
        Self {
            map,
            stars: None,
            mods: 0,
            score: None,
            passed_objects: None,
        }
    }

    /// Provide the result of a previous difficulty or performance calculation.
    /// If you already calculated the attributes for the current map-mod combination,
    /// be sure to put them in here so that they don't have to be recalculated.
    #[inline]
    pub fn attributes(mut self, attributes: impl ManiaAttributeProvider) -> Self {
        if let Some(stars) = attributes.attributes() {
            self.stars = Some(stars);
        }

        self
    }

    /// Specify mods through their bit values.
    ///
    /// See [https://github.com/ppy/osu-api/wiki#mods](https://github.com/ppy/osu-api/wiki#mods)
    #[inline]
    pub fn mods(mut self, mods: u32) -> Self {
        self.mods = mods;

        self
    }

    /// Specify the score of a play.
    /// On `NoMod` its between 0 and 1,000,000, on `Easy` between 0 and 500,000, etc.
    #[inline]
    pub fn score(mut self, score: u32) -> Self {
        self.score = Some(score as f64);

        self
    }

    /// Amount of passed objects for partial plays, e.g. a fail.
    ///
    /// Be sure you also set [`score`](ManiaPP::score) or the final values
    /// won't be correct because it will incorrectly assume a score of 1,000,000.
    ///
    /// If you want to calculate the performance after every few objects, instead of
    /// using [`ManiaPP`] multiple times with different `passed_objects`, you should use
    /// [`ManiaGradualPerformanceAttributes`](crate::mania::ManiaGradualPerformanceAttributes).
    #[inline]
    pub fn passed_objects(mut self, passed_objects: usize) -> Self {
        self.passed_objects.replace(passed_objects);

        self
    }

    /// Calculate all performance related values, including pp and stars.
    pub fn calculate(self) -> ManiaPerformanceAttributes {
        let stars = self.stars.unwrap_or_else(|| {
            ManiaStars::new(self.map)
                .mods(self.mods)
                .passed_objects(self.passed_objects.unwrap_or(usize::MAX))
                .calculate()
                .stars
        });

        let ez = self.mods.ez();
        let nf = self.mods.nf();
        let ht = self.mods.ht();

        let mut scaled_score = self.score.map_or(1_000_000.0, |score| {
            score / 0.5_f64.powi(ez as i32 + nf as i32 + ht as i32)
        });

        if let Some(passed_objects) = self.passed_objects {
            let percent_passed =
                passed_objects as f64 / (self.map.n_circles + self.map.n_sliders) as f64;

            scaled_score /= percent_passed;
        }

        let mut od = 34.0 + 3.0 * (10.0 - self.map.od as f64).clamp(0.0, 10.0);
        let clock_rate = self.mods.clock_rate();

        let mut multiplier = 0.8;

        if nf {
            multiplier *= 0.9;
        }

        if ez {
            multiplier *= 0.5;
            od *= 1.4;
        }

        let hit_window = ((od * clock_rate).floor() / clock_rate).ceil();

        let strain_value = self.compute_strain(scaled_score, stars);
        let acc_value = self.compute_accuracy_value(scaled_score, strain_value, hit_window);

        let pp = (strain_value.powf(1.1) + acc_value.powf(1.1)).powf(1.0 / 1.1) * multiplier;

        ManiaPerformanceAttributes {
            difficulty: ManiaDifficultyAttributes { stars },
            pp_acc: acc_value,
            pp_strain: strain_value,
            pp,
        }
    }

    fn compute_strain(&self, score: f64, stars: f64) -> f64 {
        let mut strain_value = (5.0 * (stars / 0.2).max(1.0) - 4.0).powf(2.2) / 135.0;

        strain_value *= 1.0 + 0.1 * (self.map.hit_objects.len() as f64 / 1500.0).min(1.0);

        if score <= 500_000.0 {
            strain_value = 0.0;
        } else if score <= 600_000.0 {
            strain_value *= (score - 500_000.0) / 100_000.0 * 0.3;
        } else if score <= 700_000.0 {
            strain_value *= 0.3 + (score - 600_000.0) / 100_000.0 * 0.25;
        } else if score <= 800_000.0 {
            strain_value *= 0.55 + (score - 700_000.0) / 100_000.0 * 0.2;
        } else if score <= 900_000.0 {
            strain_value *= 0.75 + (score - 800_000.0) / 100_000.0 * 0.15;
        } else {
            strain_value *= 0.9 + (score - 900_000.0) / 100_000.0 * 0.1;
        }

        strain_value
    }

    #[inline]
    fn compute_accuracy_value(&self, score: f64, strain: f64, hit_window: f64) -> f64 {
        (0.2 - (hit_window - 34.0) * 0.006667).max(0.0)
            * strain
            * ((score - 960_000.0).max(0.0) / 40_000.0).powf(1.1)
    }
}

/// Abstract type to provide flexibility when passing difficulty attributes to a performance calculation.
pub trait ManiaAttributeProvider {
    /// Provide the star rating (only difficulty attribute for osu!mania).
    fn attributes(self) -> Option<f64>;
}

impl ManiaAttributeProvider for f64 {
    #[inline]
    fn attributes(self) -> Option<f64> {
        Some(self)
    }
}

impl ManiaAttributeProvider for ManiaDifficultyAttributes {
    #[inline]
    fn attributes(self) -> Option<f64> {
        Some(self.stars)
    }
}

impl ManiaAttributeProvider for ManiaPerformanceAttributes {
    #[inline]
    fn attributes(self) -> Option<f64> {
        Some(self.difficulty.stars)
    }
}
