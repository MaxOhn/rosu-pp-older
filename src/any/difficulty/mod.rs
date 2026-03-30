use rosu_pp::{any::InspectDifficulty, Difficulty, GameMods};

use crate::util::mods::GameModsExt;

pub mod object;
pub mod skills;

pub(crate) trait DifficultyExt {
    fn get_inspect(&self) -> InspectDifficulty;

    fn get_mods(&self) -> GameMods;

    fn get_clock_rate(&self) -> f64;

    fn get_passed_objects(&self) -> usize;

    fn get_hardrock_offsets(&self) -> bool;

    fn get_lazer(&self) -> bool;
}

impl DifficultyExt for Difficulty {
    fn get_inspect(&self) -> InspectDifficulty {
        self.to_owned().inspect()
    }

    fn get_mods(&self) -> GameMods {
        self.get_inspect().mods
    }

    fn get_clock_rate(&self) -> f64 {
        let difficulty = self.get_inspect();

        difficulty
            .clock_rate
            .unwrap_or(difficulty.mods.clock_rate())
    }

    fn get_passed_objects(&self) -> usize {
        self.get_inspect()
            .passed_objects
            .map_or(usize::MAX, |n| n as usize)
    }

    fn get_hardrock_offsets(&self) -> bool {
        let difficulty = self.get_inspect();

        difficulty
            .hardrock_offsets
            .unwrap_or_else(|| difficulty.mods.hardrock_offsets())
    }

    fn get_lazer(&self) -> bool {
        self.get_inspect().lazer.unwrap_or(true)
    }
}
