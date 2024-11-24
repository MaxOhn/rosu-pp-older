/// The result of a difficulty calculation on an osu!catch map.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CatchDifficultyAttributes {
    /// The final star rating
    pub stars: f64,
    /// The approach rate.
    pub ar: f64,
    /// The amount of fruits.
    pub n_fruits: u32,
    /// The amount of droplets.
    pub n_droplets: u32,
    /// The amount of tiny droplets.
    pub n_tiny_droplets: u32,
    /// Whether the [`Beatmap`] was a convert i.e. an osu!standard map.
    ///
    /// [`Beatmap`]: crate::model::beatmap::Beatmap
    pub is_convert: bool,
}

impl CatchDifficultyAttributes {
    /// Return the maximum combo.
    pub const fn max_combo(&self) -> u32 {
        self.n_fruits + self.n_droplets
    }

    /// Whether the [`Beatmap`] was a convert i.e. an osu!standard map.
    ///
    /// [`Beatmap`]: crate::model::beatmap::Beatmap
    pub const fn is_convert(&self) -> bool {
        self.is_convert
    }

    pub(crate) fn set_object_count(&mut self, count: &ObjectCount) {
        self.n_fruits = count.fruits;
        self.n_droplets = count.droplets;
        self.n_tiny_droplets = count.tiny_droplets;
    }
}

/// The result of a performance calculation on an osu!catch map.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CatchPerformanceAttributes {
    /// The difficulty attributes that were used for the performance calculation
    pub difficulty: CatchDifficultyAttributes,
    /// The final performance points.
    pub pp: f64,
}

impl CatchPerformanceAttributes {
    /// Return the star value.
    pub const fn stars(&self) -> f64 {
        self.difficulty.stars
    }

    /// Return the performance point value.
    pub const fn pp(&self) -> f64 {
        self.pp
    }

    /// Return the maximum combo of the map.
    pub const fn max_combo(&self) -> u32 {
        self.difficulty.max_combo()
    }

    /// Whether the [`Beatmap`] was a convert i.e. an osu!standard map.
    ///
    /// [`Beatmap`]: crate::model::beatmap::Beatmap
    pub const fn is_convert(&self) -> bool {
        self.difficulty.is_convert
    }
}

impl From<CatchPerformanceAttributes> for CatchDifficultyAttributes {
    fn from(attributes: CatchPerformanceAttributes) -> Self {
        attributes.difficulty
    }
}

#[derive(Clone, Default)]
pub struct ObjectCount {
    fruits: u32,
    droplets: u32,
    tiny_droplets: u32,
}

pub struct ObjectCountBuilder {
    count: ObjectCount,
    take: usize,
}

impl ObjectCountBuilder {
    pub fn new(take: usize) -> Self {
        Self {
            count: ObjectCount::default(),
            take,
        }
    }

    pub fn into_regular(self) -> ObjectCount {
        self.count
    }

    pub fn record_fruit(&mut self) {
        if self.take > 0 {
            self.take -= 1;
            self.count.fruits += 1;
        }
    }

    pub fn record_droplet(&mut self) {
        if self.take > 0 {
            self.take -= 1;
            self.count.droplets += 1;
        }
    }

    pub fn record_tiny_droplets(&mut self, n: u32) {
        if self.take > 0 {
            self.count.tiny_droplets += n;
        }
    }
}
