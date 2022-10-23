mod pp;
mod strain;

pub use pp::*;
use strain::Strain;

use rosu_pp::{parse::HitObject, Beatmap, Mods};

const SECTION_LEN: f32 = 400.0;
const STAR_SCALING_FACTOR: f32 = 0.018;

/// Star calculation for osu!mania maps
///
/// In case of a partial play, e.g. a fail, one can specify the amount of passed objects.
pub fn stars(
    map: &Beatmap,
    mods: impl Mods,
    passed_objects: Option<usize>,
) -> ManiaDifficultyAttributes {
    let take = passed_objects.unwrap_or(map.hit_objects.len());

    if take < 2 {
        return ManiaDifficultyAttributes::default();
    }

    let columns = map.cs.round().max(1.0) as u8;

    let clock_rate = mods.clock_rate() as f32;
    let section_len = SECTION_LEN * clock_rate;
    let mut strain = Strain::new(columns);

    let mut hit_objects = map
        .hit_objects
        .iter()
        .take(take)
        .skip(1)
        .zip(map.hit_objects.iter())
        .map(|(base, prev)| DifficultyHitObject::new(base, prev, map.cs, clock_rate));

    // No strain for first object
    let mut current_section_end =
        (map.hit_objects[0].start_time as f32 / section_len).ceil() * section_len;

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

    ManiaDifficultyAttributes { stars }
}

#[derive(Debug)]
pub(crate) struct DifficultyHitObject<'o> {
    base: &'o HitObject,
    column: usize,
    delta: f32,
}

impl<'o> DifficultyHitObject<'o> {
    #[inline]
    fn new(base: &'o HitObject, prev: &'o HitObject, cs: f32, clock_rate: f32) -> Self {
        let x_divisor = 512.0 / cs;
        let column = (base.pos.x / x_divisor).floor() as usize;

        Self {
            base,
            column,
            delta: (base.start_time as f32 - prev.start_time as f32) / clock_rate,
        }
    }
}

#[derive(Default)]
pub struct ManiaDifficultyAttributes {
    pub stars: f64,
}

pub struct ManiaPerformanceAttributes {
    pub difficulty: ManiaDifficultyAttributes,
    pub pp: f64,
    pub pp_acc: f64,
    pub pp_strain: f64,
}
