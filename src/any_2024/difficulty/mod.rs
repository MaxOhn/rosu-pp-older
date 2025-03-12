use std::num::NonZeroU64;

use rosu_pp::model::mods::{reexports::GameModsLegacy, GameMods};

use crate::util::mods::Mods;

pub mod object;
pub mod skills;

/// Difficulty calculator on maps of any mode.
///
/// # Example
///
/// ```
/// use rosu_pp::{Beatmap, Difficulty, any::DifficultyAttributes};
///
/// let map = Beatmap::from_path("./resources/2118524.osu").unwrap();
///
/// let attrs: DifficultyAttributes = Difficulty::new()
///     .mods(8 + 1024) // HDFL
///     .calculate(&map);
/// ```
#[derive(Clone, PartialEq)]
#[must_use]
pub struct Difficulty {
    mods: GameMods,
    passed_objects: Option<u32>,
    /// Clock rate will be clamped internally between 0.01 and 100.0.
    ///
    /// Since its minimum value is 0.01, its bits are never zero.
    ///
    /// This allows for an optimization to reduce the struct size by storing its
    /// bits as a [`NonZeroU64`].
    clock_rate: Option<NonZeroU64>,
    lazer: Option<bool>,
}

impl Difficulty {
    /// Create a new difficulty calculator.
    pub const fn new() -> Self {
        Self {
            mods: GameMods::Legacy(GameModsLegacy::NoMod),
            passed_objects: None,
            clock_rate: None,
            lazer: None,
        }
    }

    pub(crate) fn as_rosu(&self) -> rosu_pp::Difficulty {
        let mut difficulty = rosu_pp::Difficulty::new().mods(self.mods.clone());

        if let Some(passed_objects) = self.passed_objects {
            difficulty = difficulty.passed_objects(passed_objects);
        }

        if let Some(clock_rate) = self.clock_rate {
            difficulty = difficulty.clock_rate(non_zero_u64_to_f64(clock_rate));
        }

        if let Some(lazer) = self.lazer {
            difficulty = difficulty.lazer(lazer);
        }

        difficulty
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
    pub fn mods(self, mods: impl Into<GameMods>) -> Self {
        Self {
            mods: mods.into(),
            ..self
        }
    }

    /// Amount of passed objects for partial plays, e.g. a fail.
    pub const fn passed_objects(mut self, passed_objects: u32) -> Self {
        self.passed_objects = Some(passed_objects);

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
    pub fn clock_rate(self, clock_rate: f64) -> Self {
        let clock_rate = clock_rate.clamp(0.01, 100.0).to_bits();

        // SAFETY: The minimum value is 0.01 so its bits can never be fully
        // zero.
        let non_zero = unsafe { NonZeroU64::new_unchecked(clock_rate) };

        Self {
            clock_rate: Some(non_zero),
            ..self
        }
    }

    /// Whether the calculated attributes belong to an osu!lazer or osu!stable
    /// score.
    ///
    /// Defaults to `true`.
    pub const fn lazer(mut self, lazer: bool) -> Self {
        self.lazer = Some(lazer);

        self
    }

    pub(crate) const fn get_mods(&self) -> &GameMods {
        &self.mods
    }

    pub(crate) fn get_clock_rate(&self) -> f64 {
        self.clock_rate
            .map_or(self.mods.clock_rate(), non_zero_u64_to_f64)
    }

    pub(crate) fn get_passed_objects(&self) -> usize {
        self.passed_objects.map_or(usize::MAX, |n| n as usize)
    }

    pub(crate) fn get_lazer(&self) -> bool {
        self.lazer.unwrap_or(true)
    }
}

const fn non_zero_u64_to_f64(n: NonZeroU64) -> f64 {
    f64::from_bits(n.get())
}

impl Default for Difficulty {
    fn default() -> Self {
        Self::new()
    }
}
