use rosu_pp::{any::ModsDependent, Difficulty, GameMods};

use crate::util::mods::GameModsExt;

pub mod object;
pub mod skills;

pub(crate) trait DifficultyExt {
    fn get_mods(&self) -> GameMods;

    fn get_clock_rate(&self) -> f64;

    fn get_passed_objects(&self) -> usize;

    fn get_ar(&self) -> Option<ModsDependent>;

    fn get_cs(&self) -> Option<ModsDependent>;

    fn get_hp(&self) -> Option<ModsDependent>;

    fn get_od(&self) -> Option<ModsDependent>;

    fn get_hardrock_offsets(&self) -> bool;

    fn get_lazer(&self) -> bool;
}

impl DifficultyExt for Difficulty {
    fn get_mods(&self) -> GameMods {
        self.inspect().mods
    }

    fn get_clock_rate(&self) -> f64 {
        let difficulty = self.inspect();

        difficulty
            .clock_rate
            .unwrap_or(difficulty.mods.clock_rate())
    }

    fn get_passed_objects(&self) -> usize {
        self.inspect()
            .passed_objects
            .map_or(usize::MAX, |n| n as usize)
    }

    fn get_ar(&self) -> Option<ModsDependent> {
        self.inspect().ar
    }

    fn get_cs(&self) -> Option<ModsDependent> {
        self.inspect().cs
    }

    fn get_hp(&self) -> Option<ModsDependent> {
        self.inspect().hp
    }

    fn get_od(&self) -> Option<ModsDependent> {
        self.inspect().od
    }

    fn get_hardrock_offsets(&self) -> bool {
        let difficulty = self.inspect();

        difficulty
            .hardrock_offsets
            .unwrap_or_else(|| difficulty.mods.hardrock_offsets())
    }

    fn get_lazer(&self) -> bool {
        self.inspect().lazer.unwrap_or(true)
    }
}
