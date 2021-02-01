mod control_point_iter;
use control_point_iter::{ControlPoint, ControlPointIter};

mod curve;
use curve::Curve;

mod difficulty_object;
use difficulty_object::DifficultyObject;

mod math_util;

mod osu_object;
use osu_object::OsuObject;

mod pp;
pub use pp::{OsuAttributeProvider, OsuPP};

mod skill;
use skill::Skill;

mod skill_kind;
use skill_kind::SkillKind;

mod slider_state;
use slider_state::SliderState;

mod stars;
use stars::stars;

use rosu_pp::{Beatmap, Mods, PpResult, StarResult};

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
