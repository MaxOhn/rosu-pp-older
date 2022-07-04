mod curve;
mod difficulty_object;
mod osu_object;
mod pp;
mod skill;
mod skill_kind;
mod slider_state;

use difficulty_object::DifficultyObject;
use osu_object::OsuObject;
pub use pp::{OsuAttributeProvider, OsuPP};
use skill::Skill;
use skill_kind::SkillKind;
use slider_state::SliderState;

use rosu_pp::{osu::OsuDifficultyAttributes, Beatmap, Mods};

use self::curve::CurveBuffers;

const OBJECT_RADIUS: f32 = 64.0;
const SECTION_LEN: f32 = 400.0;
const DIFFICULTY_MULTIPLIER: f32 = 0.0675;
const NORMALIZED_RADIUS: f32 = 52.0;

/// Star calculation for osu!standard maps.
///
/// Slider paths are considered but stack leniency is ignored.
/// As most maps don't even make use of leniency and even if,
/// it has generally little effect on stars, the results are close to perfect.
/// This version is considerably more efficient than `all_included` since
/// processing stack leniency is relatively expensive.
///
/// In case of a partial play, e.g. a fail, one can specify the amount of passed objects.
pub fn stars(
    map: &Beatmap,
    mods: impl Mods,
    passed_objects: Option<usize>,
) -> OsuDifficultyAttributes {
    let take = passed_objects.unwrap_or_else(|| map.hit_objects.len());

    let map_attributes = map.attributes().mods(mods);
    let hitwindow =
        difficulty_range_od(map_attributes.od as f32).floor() / map_attributes.clock_rate as f32;
    let od = (80.0 - hitwindow) / 6.0;

    let mut diff_attributes = OsuDifficultyAttributes {
        ar: map_attributes.ar,
        od: od as f64,
        ..Default::default()
    };

    if take < 2 {
        return diff_attributes;
    }

    let radius = OBJECT_RADIUS * (1.0 - 0.7 * (map_attributes.cs as f32 - 5.0) / 5.0) / 2.0;
    let mut scaling_factor = NORMALIZED_RADIUS / radius;

    if radius < 30.0 {
        let small_circle_bonus = (30.0 - radius).min(5.0) / 50.0;
        scaling_factor *= 1.0 + small_circle_bonus;
    }

    let mut slider_state = SliderState::new(map);
    let mut ticks_buf = Vec::new();
    let mut curve_bufs = CurveBuffers::default();

    let mut hit_objects = map
        .hit_objects
        .iter()
        .take(take)
        .filter_map(|h| {
            OsuObject::new(
                h,
                map,
                radius,
                scaling_factor,
                &mut ticks_buf,
                &mut diff_attributes,
                &mut slider_state,
                &mut curve_bufs,
            )
        })
        .map(|mut h| {
            h.time /= map_attributes.clock_rate as f32;

            h
        });

    let mut aim = Skill::new(SkillKind::Aim);
    let mut speed = Skill::new(SkillKind::Speed);

    let mut prev_prev = None;
    let mut prev = hit_objects.next().unwrap();
    let mut prev_vals = None;

    // First object has no predecessor and thus no strain, handle distinctly
    let mut current_section_end = (prev.time / SECTION_LEN).ceil() * SECTION_LEN;

    // Handle second object separately to remove later if-branching
    let curr = hit_objects.next().unwrap();
    let h = DifficultyObject::new(&curr, &prev, prev_vals, prev_prev, scaling_factor);

    while h.base.time > current_section_end {
        current_section_end += SECTION_LEN;
    }

    aim.process(&h);
    speed.process(&h);

    prev_prev = Some(prev);
    prev_vals = Some((h.jump_dist, h.strain_time));
    prev = curr;

    // Handle all other objects
    for curr in hit_objects {
        let h = DifficultyObject::new(&curr, &prev, prev_vals, prev_prev, scaling_factor);

        while h.base.time > current_section_end {
            aim.save_current_peak();
            aim.start_new_section_from(current_section_end);
            speed.save_current_peak();
            speed.start_new_section_from(current_section_end);

            current_section_end += SECTION_LEN;
        }

        aim.process(&h);
        speed.process(&h);

        prev_prev = Some(prev);
        prev_vals = Some((h.jump_dist, h.strain_time));
        prev = curr;
    }

    aim.save_current_peak();
    speed.save_current_peak();

    let aim_rating = aim.difficulty_value().sqrt() * DIFFICULTY_MULTIPLIER;
    let speed_rating = speed.difficulty_value().sqrt() * DIFFICULTY_MULTIPLIER;

    let stars = aim_rating + speed_rating + (aim_rating - speed_rating).abs() / 2.0;

    diff_attributes.n_circles = map.n_circles as usize;
    diff_attributes.n_spinners = map.n_spinners as usize;
    diff_attributes.stars = stars as f64;
    diff_attributes.speed_strain = speed_rating as f64;
    diff_attributes.aim_strain = aim_rating as f64;

    diff_attributes
}

fn lerp(start: f32, end: f32, percent: f32) -> f32 {
    start + (end - start) * percent
}

#[inline]
fn difficulty_range(val: f32, max: f32, avg: f32, min: f32) -> f32 {
    if val > 5.0 {
        avg + (max - avg) * (val - 5.0) / 5.0
    } else if val < 5.0 {
        avg - (avg - min) * (5.0 - val) / 5.0
    } else {
        avg
    }
}

const OSU_OD_MAX: f32 = 20.0;
const OSU_OD_AVG: f32 = 50.0;
const OSU_OD_MIN: f32 = 80.0;

#[inline]
fn difficulty_range_od(od: f32) -> f32 {
    difficulty_range(od, OSU_OD_MAX, OSU_OD_AVG, OSU_OD_MIN)
}
