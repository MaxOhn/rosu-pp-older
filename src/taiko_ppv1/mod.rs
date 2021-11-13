mod difficulty_object;
mod pp;
mod rim;
mod strain;

use difficulty_object::DifficultyObject;
pub use pp::*;
use strain::Strain;

use rosu_pp::{Beatmap, Mods, taiko::TaikoDifficultyAttributes};

const SECTION_LEN: f32 = 400.0;

const STAR_SCALING_FACTOR: f32 = 0.04125;

/// Star calculation for osu!taiko maps.
///
/// In case of a partial play, e.g. a fail, one can specify the amount of passed objects.
pub fn stars(map: &Beatmap, mods: impl Mods, passed_objects: Option<usize>) -> TaikoDifficultyAttributes {
    let take = passed_objects.unwrap_or_else(|| map.hit_objects.len());

    if take < 2 {
        return TaikoDifficultyAttributes { stars: 0.0 };
    }

    let clock_rate = mods.speed() as f32;
    let section_len = SECTION_LEN * clock_rate;

    // No strain for first object
    let mut current_section_end =
        (map.hit_objects[0].start_time as f32 / section_len).ceil() * section_len;

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

    while h.base.start_time as f32 > current_section_end {
        current_section_end += section_len;
    }

    strain.process(&h);

    // Handle all other objects
    for h in hit_objects {
        while h.base.start_time as f32 > current_section_end {
            strain.save_current_peak();
            strain.start_new_section_from(current_section_end);

            current_section_end += section_len;
        }

        strain.process(&h);
    }

    strain.save_current_peak();

    let stars = (strain.difficulty_value() * STAR_SCALING_FACTOR) as f64;

    TaikoDifficultyAttributes { stars }
}
