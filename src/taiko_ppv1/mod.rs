mod difficulty_object;
mod pp;
mod rim;
mod strain;

use difficulty_object::DifficultyObject;
pub use pp::*;
use rosu_pp::{model::hit_object::HitObject, Beatmap};
use strain::Strain;

use crate::util::mods::Mods;

const SECTION_LEN: f32 = 400.0;

const STAR_SCALING_FACTOR: f32 = 0.04125;

/// Star calculation for osu!taiko maps.
pub fn stars(map: &Beatmap, mods: u32) -> TaikoDifficultyAttributes {
    let max_combo = map.hit_objects.iter().map(HitObject::is_circle).count() as u32;

    if map.hit_objects.len() < 2 {
        return TaikoDifficultyAttributes {
            stars: 0.0,
            max_combo,
        };
    }

    let clock_rate = mods.clock_rate() as f32;
    let section_len = SECTION_LEN * clock_rate;

    // No strain for first object
    let mut current_section_end =
        (map.hit_objects[0].start_time as f32 / section_len).ceil() * section_len;

    let mut hit_objects = map
        .hit_objects
        .iter()
        .zip(map.hit_sounds.iter())
        .skip(1)
        .zip(map.hit_objects.iter().zip(map.hit_sounds.iter()))
        .map(|(base, prev)| DifficultyObject::new(base, prev, clock_rate));

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

    TaikoDifficultyAttributes { stars, max_combo }
}

pub struct TaikoDifficultyAttributes {
    /// The final star rating.
    pub stars: f64,
    /// The maximum combo.
    pub max_combo: u32,
}

pub struct TaikoPerformanceAttributes {
    /// The difficulty attributes that were used for the performance calculation
    pub difficulty: TaikoDifficultyAttributes,
    /// The final performance points.
    pub pp: f64,
    /// The accuracy portion of the final pp.
    pub pp_acc: f64,
    /// The strain portion of the final pp.
    pub pp_strain: f64,
}

impl TaikoPerformanceAttributes {
    /// Return the maximum combo of the map.
    #[inline]
    pub fn max_combo(&self) -> u32 {
        self.difficulty.max_combo
    }
}
