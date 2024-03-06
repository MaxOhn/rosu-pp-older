mod catch_object;
mod difficulty_object;
mod movement;
mod pp;

use rosu_map::{
    section::hit_objects::{BorrowedCurve, CurveBuffers},
    util::Pos,
};
use rosu_pp::{
    catch::CatchDifficultyAttributes,
    model::{
        control_point::{DifficultyPoint, TimingPoint},
        hit_object::{HitObjectKind, Slider},
    },
    Beatmap,
};
use std::{iter::Map, vec::IntoIter};

use crate::util::{
    control_points::{difficulty_point_at, timing_point_at},
    mods::Mods,
};

pub use self::pp::*;
use self::{catch_object::CatchObject, difficulty_object::DifficultyObject, movement::Movement};

const SECTION_LENGTH: f64 = 750.0;
const STAR_SCALING_FACTOR: f64 = 0.145;

const CATCHER_SIZE: f32 = 106.75;

const LEGACY_LAST_TICK_OFFSET: f64 = 36.0;
const BASE_SCORING_DISTANCE: f64 = 100.0;

/// Star calculation for osu!ctb maps
pub fn stars(map: &Beatmap, mods: u32) -> CatchDifficultyAttributes {
    if map.hit_objects.len() < 2 {
        return CatchDifficultyAttributes::default();
    }

    let map_attributes = map.attributes().mods(mods).build();

    let attributes = CatchDifficultyAttributes {
        ar: map_attributes.ar,
        ..Default::default()
    };

    let mut params = FruitParams {
        attributes,
        curve_bufs: CurveBuffers::default(),
        last_pos: None,
        last_time: 0.0,
        ticks: Vec::new(), // using the same buffer for all sliders
        with_hr: mods.hr(),
    };

    // BUG: Incorrect object order on 2B maps that have fruits within sliders
    let mut hit_objects = map
        .hit_objects
        .iter()
        .filter_map(|h| match &h.kind {
            HitObjectKind::Circle => {
                let mut h = CatchObject::new((h.pos, h.start_time));

                if params.with_hr {
                    h = h.with_hr(&mut params);
                }

                params.attributes.n_fruits += 1;

                Some(FruitOrJuice::Fruit(Some(h)))
            }
            HitObjectKind::Slider(Slider {
                expected_dist,
                repeats,
                control_points,
                ..
            }) => {
                // HR business
                params.last_pos = Some(h.pos.x + control_points[control_points.len() - 1].pos.x);
                params.last_time = h.start_time;

                let span_count = (*repeats + 1) as f64;

                let mut tick_dist = 100.0 * map.slider_multiplier / map.slider_tick_rate;

                let beat_len = timing_point_at(&map.timing_points, h.start_time)
                    .map_or(TimingPoint::DEFAULT_BEAT_LEN, |point| point.beat_len);
                let slider_vel = difficulty_point_at(&map.difficulty_points, h.start_time)
                    .map_or(DifficultyPoint::DEFAULT_SLIDER_VELOCITY, |point| {
                        point.slider_velocity
                    });

                if map.version >= 8 {
                    tick_dist /= (100.0 / slider_vel).clamp(10.0, 1000.0) / 100.0;
                }

                // Build the curve w.r.t. the control points
                let curve =
                    BorrowedCurve::new(control_points, *expected_dist, &mut params.curve_bufs);

                let velocity =
                    (BASE_SCORING_DISTANCE * map.slider_multiplier * slider_vel) / beat_len;

                let end_time = h.start_time + span_count * curve.dist() / velocity;
                let duration = end_time - h.start_time;
                let span_duration = duration / span_count;

                // * A very lenient maximum length of a slider for ticks to be generated.
                // * This exists for edge cases such as /b/1573664 where the beatmap has
                // * been edited by the user, and should never be reached in normal usage.
                let max_len = 100_000.0;

                let len = curve.dist().min(max_len);
                tick_dist = tick_dist.clamp(0.0, len);
                let min_dist_from_end = velocity * 10.0;

                let mut curr_dist = tick_dist;
                let pixel_len = expected_dist.unwrap_or(0.0);
                let time_add = duration * tick_dist / (pixel_len * span_count);

                let target = pixel_len - tick_dist / 8.0;

                params.ticks.reserve((target / tick_dist) as usize);

                // Tick of the first span
                while curr_dist < len - min_dist_from_end {
                    let progress = curr_dist / len;
                    let pos = h.pos + curve.position_at(progress);
                    let time = h.start_time + progress * span_duration;
                    params.ticks.push((pos, time));
                    curr_dist += tick_dist;
                }

                params.attributes.n_tiny_droplets += tiny_droplet_count(
                    h.start_time,
                    time_add,
                    duration,
                    span_count as usize,
                    &params.ticks,
                );

                let mut slider_objects =
                    Vec::with_capacity(span_count as usize * (params.ticks.len() + 1));
                slider_objects.push((h.pos, h.start_time));

                // Other spans
                if *repeats == 0 {
                    slider_objects.append(&mut params.ticks); // automatically empties buffer for next slider
                } else {
                    slider_objects.extend(&params.ticks);

                    for span_idx in 1..=*repeats {
                        let progress = (span_idx % 2 == 1) as u8 as f64;
                        let pos = h.pos + curve.position_at(progress);
                        let time_offset = span_duration * span_idx as f64;

                        // Reverse tick
                        slider_objects.push((pos, h.start_time + time_offset));

                        let new_ticks = params.ticks.iter().enumerate().map(|(i, (pos, time))| {
                            (*pos, *time + time_offset + time_add * i as f64)
                        });

                        // Actual ticks
                        if span_idx & 1 == 1 {
                            slider_objects.extend(new_ticks.rev());
                        } else {
                            slider_objects.extend(new_ticks);
                        }
                    }

                    params.ticks.clear();
                }

                // Slider tail
                let progress = (*repeats % 2 == 0) as u8 as f64;
                let pos = h.pos + curve.position_at(progress);
                slider_objects.push((pos, h.start_time + duration));

                let new_fruits = *repeats + 2;
                params.attributes.n_fruits += new_fruits as u32;
                params.attributes.n_droplets += (slider_objects.len() - new_fruits) as u32;

                let iter = slider_objects
                    .into_iter()
                    .map(CatchObject::new as fn(_) -> _);

                Some(FruitOrJuice::Juice(iter))
            }
            HitObjectKind::Spinner { .. } | HitObjectKind::Hold { .. } => None,
        })
        .flatten();

    // Hyper dash business
    let base_size = calculate_catch_width(map_attributes.cs as f32) * 0.5;
    let half_catcher_width = base_size * 0.8;
    let catcher_size = base_size;

    let mut last_direction = 0;
    let mut last_excess = catcher_size;

    // Strain business
    let mut movement = Movement::new();
    let section_len = SECTION_LENGTH * map_attributes.clock_rate;
    let mut current_section_end =
        (map.hit_objects[0].start_time / section_len).ceil() * section_len;

    let mut prev = hit_objects.next().unwrap();
    let mut curr = hit_objects.next().unwrap();

    prev.init_hyper_dash(catcher_size, &curr, &mut last_direction, &mut last_excess);

    // Handle second object separately to remove later if-branching
    let next = hit_objects.next().unwrap();
    curr.init_hyper_dash(catcher_size, &next, &mut last_direction, &mut last_excess);

    let h = DifficultyObject::new(&curr, &prev, half_catcher_width, map_attributes.clock_rate);

    while h.base.time > current_section_end {
        current_section_end += section_len;
    }

    movement.process(&h);

    prev = curr;
    curr = next;

    // Handle all other objects
    for next in hit_objects {
        curr.init_hyper_dash(catcher_size, &next, &mut last_direction, &mut last_excess);

        let h = DifficultyObject::new(&curr, &prev, half_catcher_width, map_attributes.clock_rate);

        while h.base.time > current_section_end {
            movement.save_current_peak();
            movement.start_new_section_from(current_section_end);
            current_section_end += section_len;
        }

        movement.process(&h);

        prev = curr;
        curr = next;
    }

    // Same as in loop but without init_hyper_dash because `curr` is the last element
    let h = DifficultyObject::new(&curr, &prev, half_catcher_width, map_attributes.clock_rate);

    while h.base.time > current_section_end {
        movement.save_current_peak();
        movement.start_new_section_from(current_section_end);

        current_section_end += section_len;
    }

    movement.process(&h);
    movement.save_current_peak();

    params.attributes.stars = movement.difficulty_value().sqrt() * STAR_SCALING_FACTOR;

    params.attributes
}

// BUG: Sometimes there are off-by-one errors,
// presumably caused by floating point inaccuracies
fn tiny_droplet_count(
    start_time: f64,
    time_between_ticks: f64,
    duration: f64,
    span_count: usize,
    ticks: &[(Pos, f64)],
) -> u32 {
    // tiny droplets preceeding a _tick_
    let per_tick = if !ticks.is_empty() && time_between_ticks > 80.0 {
        let time_between_tiny = shrink_down(time_between_ticks);

        // add a little for floating point inaccuracies
        let start = time_between_tiny + 0.001;

        count_iterations(start, time_between_tiny, time_between_ticks)
    } else {
        0
    };

    // tiny droplets preceeding a _reverse_
    let last = ticks.last().map_or(start_time, |(_, last)| *last);
    let repeat_time = start_time + duration / span_count as f64;
    let since_last_tick = repeat_time - last;

    let span_last_section = if since_last_tick > 80.0 {
        let time_between_tiny = shrink_down(since_last_tick);

        count_iterations(time_between_tiny, time_between_tiny, since_last_tick)
    } else {
        0
    };

    // tiny droplets preceeding the slider tail
    // necessary to handle distinctly because of the legacy last tick
    let last = ticks.last().map_or(start_time, |(_, last)| *last);
    let end_time = start_time + duration / span_count as f64 - LEGACY_LAST_TICK_OFFSET;
    let since_last_tick = end_time - last;

    let last_section = if since_last_tick > 80.0 {
        let time_between_tiny = shrink_down(since_last_tick);

        count_iterations(time_between_tiny, time_between_tiny, since_last_tick)
    } else {
        0
    };

    // Combine tiny droplets counts
    per_tick * (ticks.len() * span_count) as u32
        + span_last_section * (span_count.saturating_sub(1) as u32)
        + last_section
}

#[inline]
fn shrink_down(mut val: f64) -> f64 {
    while val > 100.0 {
        val /= 2.0;
    }

    val
}

#[inline]
fn count_iterations(mut start: f64, step: f64, end: f64) -> u32 {
    let mut count = 0;

    while start < end {
        count += 1;
        start += step;
    }

    count
}

#[inline]
fn calculate_catch_width(cs: f32) -> f32 {
    CATCHER_SIZE * (1.0 - 0.7 * (cs - 5.0) / 5.0).abs()
}

type JuiceStream = Map<IntoIter<(Pos, f64)>, fn((Pos, f64)) -> CatchObject>;

pub(crate) enum FruitOrJuice {
    Fruit(Option<CatchObject>),
    Juice(JuiceStream),
}

impl Iterator for FruitOrJuice {
    type Item = CatchObject;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Fruit(fruit) => fruit.take(),
            Self::Juice(slider) => slider.next(),
        }
    }

    #[inline]
    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            Self::Fruit(Some(_)) => (1, Some(1)),
            Self::Fruit(None) => (0, Some(0)),
            Self::Juice(slider) => slider.size_hint(),
        }
    }
}

pub(crate) struct FruitParams {
    pub(crate) attributes: CatchDifficultyAttributes,
    pub(crate) curve_bufs: CurveBuffers,
    pub(crate) last_pos: Option<f32>,
    pub(crate) last_time: f64,
    pub(crate) ticks: Vec<(Pos, f64)>,
    pub(crate) with_hr: bool,
}
