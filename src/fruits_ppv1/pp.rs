use super::stars;

use rosu_pp::{
    catch::{CatchDifficultyAttributes, CatchPerformanceAttributes},
    Beatmap, DifficultyAttributes, Mods, PerformanceAttributes,
};

/// Calculator for pp on osu!ctb maps.
///
/// # Example
///
/// ```
/// # use rosu_pp::{CatchPP, Beatmap};
/// # /*
/// let map: Beatmap = ...
/// # */
/// # let map = Beatmap::default();
/// let attrs = CatchPP::new(&map)
///     .mods(8 + 64) // HDDT
///     .combo(1234)
///     .misses(1)
///     .accuracy(98.5)
///     .calculate();
///
/// println!("PP: {} | Stars: {}", attrs.pp(), attrs.stars());
///
/// let next_result = CatchPP::new(&map)
///     .attributes(attrs) // reusing previous results for performance
///     .mods(8 + 64)      // has to be the same to reuse attributes
///     .accuracy(99.5)
///     .calculate();
///
/// println!("PP: {} | Stars: {}", next_result.pp(), next_result.stars());
/// ```
#[derive(Clone, Debug)]
pub struct FruitsPP<'m> {
    map: &'m Beatmap,
    attributes: Option<CatchDifficultyAttributes>,
    mods: u32,
    combo: Option<usize>,

    n_fruits: Option<usize>,
    n_droplets: Option<usize>,
    n_tiny_droplets: Option<usize>,
    n_tiny_droplet_misses: Option<usize>,
    n_misses: usize,
    passed_objects: Option<usize>,
}

impl<'m> FruitsPP<'m> {
    #[inline]
    pub fn new(map: &'m Beatmap) -> Self {
        Self {
            map,
            attributes: None,
            mods: 0,
            combo: None,

            n_fruits: None,
            n_droplets: None,
            n_tiny_droplets: None,
            n_tiny_droplet_misses: None,
            n_misses: 0,
            passed_objects: None,
        }
    }

    /// [`CatchAttributeProvider`] is implemented by [`DifficultyAttributes`](crate::catch::DifficultyAttributes),
    /// and by [`StarResult`](crate::StarResult) meaning you can give the
    /// result of a star calculation or a pp calculation.
    /// If you already calculated the attributes for the current map-mod combination,
    /// be sure to put them in here so that they don't have to be recalculated.
    #[inline]
    pub fn attributes(mut self, attributes: impl CatchAttributeProvider) -> Self {
        if let Some(attributes) = attributes.attributes() {
            self.attributes.replace(attributes);
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

    /// Specify the max combo of the play.
    #[inline]
    pub fn combo(mut self, combo: usize) -> Self {
        self.combo.replace(combo);

        self
    }

    /// Specify the amount of fruits of a play i.e. n300.
    #[inline]
    pub fn fruits(mut self, n_fruits: usize) -> Self {
        self.n_fruits.replace(n_fruits);

        self
    }

    /// Specify the amount of droplets of a play i.e. n100.
    #[inline]
    pub fn droplets(mut self, n_droplets: usize) -> Self {
        self.n_droplets.replace(n_droplets);

        self
    }

    /// Specify the amount of tiny droplets of a play i.e. n50.
    #[inline]
    pub fn tiny_droplets(mut self, n_tiny_droplets: usize) -> Self {
        self.n_tiny_droplets.replace(n_tiny_droplets);

        self
    }

    /// Specify the amount of tiny droplet misses of a play i.e. n_katu.
    #[inline]
    pub fn tiny_droplet_misses(mut self, n_tiny_droplet_misses: usize) -> Self {
        self.n_tiny_droplet_misses.replace(n_tiny_droplet_misses);

        self
    }

    /// Specify the amount of fruit / droplet misses of the play.
    #[inline]
    pub fn misses(mut self, n_misses: usize) -> Self {
        self.n_misses = n_misses;

        self
    }

    /// Amount of passed objects for partial plays, e.g. a fail.
    #[inline]
    pub fn passed_objects(mut self, passed_objects: usize) -> Self {
        self.passed_objects.replace(passed_objects);

        self
    }

    /// Generate the hit results with respect to the given accuracy between `0` and `100`.
    ///
    /// Be sure to set `misses` beforehand! Also, if available, set `attributes` beforehand.
    pub fn accuracy(mut self, mut acc: f32) -> Self {
        if self.attributes.is_none() {
            self.attributes
                .replace(stars(self.map, self.mods, self.passed_objects));
        }

        let attributes = self.attributes.as_ref().unwrap();
        let max_combo = attributes.max_combo();

        let n_droplets = self
            .n_droplets
            .unwrap_or_else(|| attributes.n_droplets.saturating_sub(self.n_misses));

        let n_fruits = self.n_fruits.unwrap_or_else(|| {
            max_combo
                .saturating_sub(self.n_misses)
                .saturating_sub(n_droplets)
        });

        let max_tiny_droplets = attributes.n_tiny_droplets;
        acc /= 100.0;

        let n_tiny_droplets = self.n_tiny_droplets.unwrap_or_else(|| {
            ((acc * (max_combo + max_tiny_droplets) as f32).round() as usize)
                .saturating_sub(n_fruits)
                .saturating_sub(n_droplets)
        });

        let n_tiny_droplet_misses = max_tiny_droplets.saturating_sub(n_tiny_droplets);

        self.n_fruits.replace(n_fruits);
        self.n_droplets.replace(n_droplets);
        self.n_tiny_droplets.replace(n_tiny_droplets);
        self.n_tiny_droplet_misses.replace(n_tiny_droplet_misses);

        self
    }

    fn assert_hitresults(&mut self, attributes: &CatchDifficultyAttributes) {
        let max_combo = attributes.max_combo();

        let correct_combo_hits = self
            .n_fruits
            .and_then(|f| self.n_droplets.map(|d| f + d + self.n_misses))
            .filter(|h| *h == max_combo);

        let correct_fruits = self
            .n_fruits
            .filter(|f| *f >= attributes.n_fruits.saturating_sub(self.n_misses));

        let correct_droplets = self
            .n_droplets
            .filter(|d| *d >= attributes.n_droplets.saturating_sub(self.n_misses));

        let correct_tinies = self
            .n_tiny_droplets
            .and_then(|t| self.n_tiny_droplet_misses.map(|m| t + m))
            .filter(|h| *h == attributes.n_tiny_droplets);

        if correct_combo_hits
            .and(correct_fruits)
            .and(correct_droplets)
            .and(correct_tinies)
            .is_none()
        {
            let mut n_fruits = self.n_fruits.unwrap_or(0);
            let mut n_droplets = self.n_droplets.unwrap_or(0);
            let mut n_tiny_droplets = self.n_tiny_droplets.unwrap_or(0);
            let n_tiny_droplet_misses = self.n_tiny_droplet_misses.unwrap_or(0);

            let missing = max_combo
                .saturating_sub(n_fruits)
                .saturating_sub(n_droplets)
                .saturating_sub(self.n_misses);

            let missing_fruits =
                missing.saturating_sub(attributes.n_droplets.saturating_sub(n_droplets));

            n_fruits += missing_fruits;
            n_droplets += missing.saturating_sub(missing_fruits);
            n_tiny_droplets += attributes
                .n_tiny_droplets
                .saturating_sub(n_tiny_droplets)
                .saturating_sub(n_tiny_droplet_misses);

            self.n_fruits.replace(n_fruits);
            self.n_droplets.replace(n_droplets);
            self.n_tiny_droplets.replace(n_tiny_droplets);
            self.n_tiny_droplet_misses.replace(n_tiny_droplet_misses);
        }
    }

    /// Returns an object which contains the pp and [`DifficultyAttributes`](crate::catch::DifficultyAttributes)
    /// containing stars and other attributes.
    pub fn calculate(mut self) -> CatchPerformanceAttributes {
        let attributes = self
            .attributes
            .take()
            .unwrap_or_else(|| stars(self.map, self.mods, self.passed_objects));

        let max_combo = attributes.max_combo();

        // Make sure all objects are set
        self.assert_hitresults(&attributes);

        let stars = attributes.stars;

        // Relying heavily on aim
        let mut pp = (5.0 * (stars as f32 / 0.0049).max(1.0) - 4.0).powi(2) / 100_000.0;

        let mut combo_hits = self.combo_hits();

        if combo_hits == 0 {
            combo_hits = max_combo;
        }

        // Longer maps are worth more
        let len_bonus = 0.95
            + 0.4 * (combo_hits as f32 / 3000.0).min(1.0)
            + (combo_hits > 3000) as u8 as f32 * (combo_hits as f32 / 3000.0).log10() * 0.5;
        pp *= len_bonus;

        // Penalize misses exponentially
        pp *= 0.97_f32.powi(self.n_misses as i32);

        // Combo scaling
        if let Some(combo) = self.combo.filter(|_| max_combo > 0) {
            pp *= (combo as f32 / max_combo as f32).powf(0.8).min(1.0);
        }

        // AR scaling
        let ar = attributes.ar;
        let mut ar_factor = 1.0;
        if ar > 9.0 {
            ar_factor += 0.1 * (ar - 9.0);
        } else if ar < 8.0 {
            ar_factor += 0.025 * (8.0 - ar);
        }
        pp *= ar_factor as f32;

        // HD bonus
        if self.mods.hd() {
            pp *= 1.05 + 0.075 * (10.0 - ar.min(10.0) as f32);
        }

        // FL bonus
        if self.mods.fl() {
            pp *= 1.35 * len_bonus;
        }

        // Accuracy scaling
        pp *= self.acc().powf(5.5);

        // NF penalty
        if self.mods.nf() {
            pp *= 0.9;
        }

        CatchPerformanceAttributes {
            difficulty: attributes,
            pp: pp as f64,
        }
    }

    #[inline]
    fn combo_hits(&self) -> usize {
        self.n_fruits.unwrap_or(0) + self.n_droplets.unwrap_or(0) + self.n_misses
    }

    #[inline]
    fn successful_hits(&self) -> usize {
        self.n_fruits.unwrap_or(0)
            + self.n_droplets.unwrap_or(0)
            + self.n_tiny_droplets.unwrap_or(0)
    }

    #[inline]
    fn total_hits(&self) -> usize {
        self.successful_hits() + self.n_tiny_droplet_misses.unwrap_or(0) + self.n_misses
    }

    #[inline]
    fn acc(&self) -> f32 {
        let total_hits = self.total_hits();

        if total_hits == 0 {
            1.0
        } else {
            (self.successful_hits() as f32 / total_hits as f32)
                .max(0.0)
                .min(1.0)
        }
    }
}

pub trait CatchAttributeProvider {
    fn attributes(self) -> Option<CatchDifficultyAttributes>;
}

impl CatchAttributeProvider for CatchDifficultyAttributes {
    #[inline]
    fn attributes(self) -> Option<CatchDifficultyAttributes> {
        Some(self)
    }
}

impl CatchAttributeProvider for DifficultyAttributes {
    #[inline]
    fn attributes(self) -> Option<CatchDifficultyAttributes> {
        #[allow(irrefutable_let_patterns)]
        if let Self::Catch(attributes) = self {
            Some(attributes)
        } else {
            None
        }
    }
}

impl CatchAttributeProvider for PerformanceAttributes {
    #[inline]
    fn attributes(self) -> Option<CatchDifficultyAttributes> {
        self.difficulty_attributes().attributes()
    }
}
