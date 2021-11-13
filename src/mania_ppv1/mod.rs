mod pp;
mod strain;

pub use pp::*;
use strain::Strain;

use rosu_pp::{mania::ManiaDifficultyAttributes, parse::HitObject, Beatmap, GameMode, Mods};

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
    let take = passed_objects.unwrap_or_else(|| map.hit_objects.len());

    if take < 2 {
        return ManiaDifficultyAttributes::default();
    }

    let rounded_cs = map.cs.round();

    let columns = match map.mode {
        GameMode::MNA => rounded_cs.max(1.0) as u8,
        GameMode::STD => {
            let rounded_od = map.od.round();

            let n_objects = map.n_circles + map.n_sliders + map.n_spinners;
            let slider_or_spinner_ratio = (n_objects - map.n_circles) as f32 / n_objects as f32;

            if slider_or_spinner_ratio < 0.2 {
                7
            } else if slider_or_spinner_ratio < 0.3 || rounded_cs >= 5.0 {
                6 + (rounded_od > 5.0) as u8
            } else if slider_or_spinner_ratio > 0.6 {
                4 + (rounded_od > 4.0) as u8
            } else {
                (rounded_od as u8 + 1).max(4).min(7)
            }
        }
        other => panic!("can not calculate mania difficulty on a {:?} map", other),
    };

    let clock_rate = mods.speed() as f32;
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
