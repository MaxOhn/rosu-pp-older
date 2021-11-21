use super::stars;

use rosu_pp::{
    mania::{ManiaDifficultyAttributes, ManiaPerformanceAttributes},
    Beatmap, DifficultyAttributes, Mods, PerformanceAttributes,
};

/// Calculator for pp on osu!mania maps.
///
/// # Example
///
/// ```
/// # use rosu_pp::{ManiaPP, PpResult, Beatmap};
/// # /*
/// let map: Beatmap = ...
/// # */
/// # let map = Beatmap::default();
/// let pp_result: PpResult = ManiaPP::new(&map)
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
pub struct ManiaPP<'m> {
    map: &'m Beatmap,
    stars: Option<f32>,
    mods: u32,
    score: Option<f32>,
    acc: f32,
    passed_objects: Option<usize>,
}

impl<'m> ManiaPP<'m> {
    #[inline]
    pub fn new(map: &'m Beatmap) -> Self {
        Self {
            map,
            stars: None,
            mods: 0,
            score: None,
            acc: 1.0,
            passed_objects: None,
        }
    }

    /// [`ManiaAttributeProvider`] is implemented by `f32`, [`StarResult`](crate::StarResult),
    /// and by [`PpResult`](crate::PpResult) meaning you can give the star rating,
    /// the result of a star calculation, or the result of a pp calculation.
    /// If you already calculated the attributes for the current map-mod combination,
    /// be sure to put them in here so that they don't have to be recalculated.
    #[inline]
    pub fn attributes(mut self, attributes: impl ManiaAttributeProvider) -> Self {
        if let Some(stars) = attributes.attributes() {
            self.stars.replace(stars);
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
        self.score.replace(score as f32);

        self
    }

    /// Specify the accuracy of a play between 0.0 and 100.0.
    #[inline]
    pub fn accuracy(mut self, acc: f32) -> Self {
        self.acc = acc / 100.0;

        self
    }

    /// Amount of passed objects for partial plays, e.g. a fail.
    #[inline]
    pub fn passed_objects(mut self, passed_objects: usize) -> Self {
        self.passed_objects.replace(passed_objects);

        self
    }

    /// Returns an object which contains the pp and stars.
    pub fn calculate(self) -> ManiaPerformanceAttributes {
        let stars = self
            .stars
            .unwrap_or_else(|| stars(self.map, self.mods, self.passed_objects).stars as f32);

        let ez = self.mods.ez();
        let nf = self.mods.nf();
        let ht = self.mods.ht();

        let mut scaled_score = self.score.map_or(1_000_000.0, |score| {
            score / 0.5_f32.powi(ez as i32 + nf as i32 + ht as i32)
        });

        if let Some(passed_objects) = self.passed_objects {
            let percent_passed =
                passed_objects as f32 / (self.map.n_circles + self.map.n_sliders) as f32;

            scaled_score /= percent_passed;
        }

        let mut multiplier = 1.1;

        if nf {
            multiplier *= 0.9;
        }

        if ez {
            multiplier *= 0.5;
        }

        let hit_window = {
            let mut od = 34.0 + 3.0 * (10.0 - self.map.od).max(0.0).min(10.0);

            if ez {
                od *= 1.4;
            } else if self.mods.hr() {
                od /= 1.4;
            }

            let clock_rate = self.mods.speed();

            ((od * clock_rate as f32).floor() / clock_rate as f32).ceil()
        };

        let strain_value = self.compute_strain(scaled_score, stars);
        let acc_value = self.compute_accuracy_value(hit_window);

        let pp = (strain_value.powf(1.1) + acc_value.powf(1.1)).powf(1.0 / 1.1) * multiplier;

        ManiaPerformanceAttributes {
            difficulty: ManiaDifficultyAttributes {
                stars: stars as f64,
            },
            pp_acc: acc_value as f64,
            pp_strain: strain_value as f64,
            pp: pp as f64,
        }
    }

    fn compute_strain(&self, score: f32, stars: f32) -> f32 {
        let mut strain_value = (5.0 * (stars / 0.0825).max(1.0) - 4.0).powi(3) / 110_000.0;

        strain_value *= 1.0 + 0.1 * (self.total_hits() as f32 / 1500.0).min(1.0);

        if score <= 500_000.0 {
            strain_value = 0.0;
        } else if score <= 600_000.0 {
            strain_value *= (score - 500_000.0) / 100_000.0 * 0.3;
        } else if score <= 700_000.0 {
            strain_value *= 0.3 + (score - 600_000.0) / 100_000.0 * 0.25;
        } else if score <= 800_000.0 {
            strain_value *= 0.65 + (score - 700_000.0) / 100_000.0 * 0.2;
        } else if score <= 900_000.0 {
            strain_value *= 0.85 + (score - 800_000.0) / 100_000.0 * 0.15;
        } else {
            strain_value *= 0.95 + (score - 900_000.0) / 100_000.0 * 0.1;
        }

        strain_value
    }

    #[inline]
    fn compute_accuracy_value(&self, hit_window: f32) -> f32 {
        let mut acc_value = (150.0 / hit_window * self.acc.powi(16)).powf(1.8) * 2.5;

        // Length bonus
        acc_value *= ((self.total_hits() as f32 / 1500.0).powf(0.3)).min(1.15);

        acc_value
    }

    #[inline]
    fn total_hits(&self) -> usize {
        self.map.hit_objects.len()
    }
}

pub trait ManiaAttributeProvider {
    fn attributes(self) -> Option<f32>;
}

impl ManiaAttributeProvider for f32 {
    #[inline]
    fn attributes(self) -> Option<f32> {
        Some(self)
    }
}

impl ManiaAttributeProvider for ManiaDifficultyAttributes {
    #[inline]
    fn attributes(self) -> Option<f32> {
        Some(self.stars as f32)
    }
}

impl ManiaAttributeProvider for DifficultyAttributes {
    #[inline]
    fn attributes(self) -> Option<f32> {
        if let Self::Mania(attributes) = self {
            Some(attributes.stars as f32)
        } else {
            None
        }
    }
}

impl ManiaAttributeProvider for PerformanceAttributes {
    #[inline]
    fn attributes(self) -> Option<f32> {
        self.difficulty_attributes().attributes()
    }
}
