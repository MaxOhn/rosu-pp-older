mod difficulty_object;
mod pp;
mod rim;
mod strain;

use difficulty_object::DifficultyObject;
pub use pp::*;
use strain::Strain;

use rosu_pp::{taiko::DifficultyAttributes, Beatmap, Mods, StarResult};

const SECTION_LEN: f32 = 400.0;

const STAR_SCALING_FACTOR: f32 = 0.04125;

/// Star calculation for osu!taiko maps.
///
/// In case of a partial play, e.g. a fail, one can specify the amount of passed objects.
pub fn stars(map: &Beatmap, mods: impl Mods, passed_objects: Option<usize>) -> StarResult {
    let take = passed_objects.unwrap_or_else(|| map.hit_objects.len());

    if take < 2 {
        return StarResult::Taiko(DifficultyAttributes { stars: 0.0 });
    }

    let clock_rate = mods.speed();
    let section_len = SECTION_LEN * clock_rate;

    // No strain for first object
    let mut current_section_end =
        (map.hit_objects[0].start_time / section_len).ceil() * section_len;

    let mut hit_objects = map
        .hit_objects
        .iter()
        .take(take)
        .enumerate()
        .skip(1)
        .zip(map.hit_objects.iter())
        .map(|((idx, base), prev)| DifficultyObject::new(idx, base, prev, clock_rate));

    let mut strain = Strain::new();

    // Handle second object separately to remove later if-branching
    let h = hit_objects.next().unwrap();

    while h.base.start_time > current_section_end {
        current_section_end += section_len;
    }

    strain.process(&h);

    // Handle all other objects
    for h in hit_objects {
        while h.base.start_time > current_section_end {
            strain.save_current_peak();
            strain.start_new_section_from(current_section_end);

            current_section_end += section_len;
        }

        strain.process(&h);
    }

    strain.save_current_peak();

    let stars = strain.difficulty_value() * STAR_SCALING_FACTOR;

    StarResult::Taiko(DifficultyAttributes { stars })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;

    #[test]
    #[ignore]
    fn taiko_ppv1_single() {
        let file = match File::open("E:/Games/osu!/beatmaps/168450.osu") {
            Ok(file) => file,
            Err(why) => panic!("Could not open file: {}", why),
        };

        let map = match Beatmap::parse(file) {
            Ok(map) => map,
            Err(why) => panic!("Error while parsing map: {}", why),
        };

        let result = TaikoPP::new(&map).mods(0).calculate();

        println!("Stars: {}", result.stars());
        println!("PP: {}", result.pp());
    }
}
