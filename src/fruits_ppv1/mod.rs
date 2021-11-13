mod catch_object;
mod control_point_iter;
mod curve;
mod difficulty_object;
mod movement;
mod pp;
mod slider_state;

use catch_object::CatchObject;
use control_point_iter::{ControlPoint, ControlPointIter};
use curve::Curve;
use difficulty_object::DifficultyObject;
use movement::Movement;
pub use pp::*;
use slider_state::SliderState;

use rosu_pp::{
    fruits::FruitsDifficultyAttributes,
    parse::{HitObjectKind, Pos2},
    Beatmap, Mods,
};

use std::convert::identity;

use self::curve::CurveBuffers;

const SECTION_LENGTH: f32 = 750.0;
const STAR_SCALING_FACTOR: f32 = 0.145;

const CATCHER_SIZE: f32 = 106.75;

const LEGACY_LAST_TICK_OFFSET: f32 = 36.0;

/// Star calculation for osu!ctb maps
///
/// In case of a partial play, e.g. a fail, one can specify the amount of passed objects.
// Slider parsing based on https://github.com/osufx/catch-the-pp
pub fn stars(
    map: &Beatmap,
    mods: impl Mods,
    passed_objects: Option<usize>,
) -> FruitsDifficultyAttributes {
    if map.hit_objects.len() < 2 {
        return FruitsDifficultyAttributes::default();
    }

    let take = passed_objects.unwrap_or(usize::MAX);

    let attributes = map.attributes().mods(mods);
    let with_hr = mods.hr();
    let mut ticks = Vec::new(); // using the same buffer for all sliders
    let mut slider_state = SliderState::new(map);

    let mut fruits = 0;
    let mut droplets = 0;
    let mut tiny_droplets = 0;

    let mut curve_bufs = CurveBuffers::default();

    // BUG: Incorrect object order on 2B maps that have fruits within sliders
    let mut hit_objects = map
        .hit_objects
        .iter()
        .scan((None, 0.0), |(last_pos, last_time), h| match &h.kind {
            HitObjectKind::Circle => {
                let mut h = CatchObject::new((h.pos, h.start_time as f32));

                if with_hr {
                    h = h.with_hr(last_pos, last_time);
                }

                fruits += 1;

                Some(Some(FruitOrJuice::Fruit(Some(h))))
            }
            HitObjectKind::Slider {
                pixel_len,
                repeats,
                control_points,
            } => {
                let pixel_len = *pixel_len as f32;

                // HR business
                last_pos.replace(
                    h.pos.x + control_points[control_points.len() - 1].pos.x
                        - control_points[0].pos.x,
                );
                *last_time = h.start_time as f32;

                // Responsible for timing point values
                slider_state.update(h.start_time as f32);

                let mut tick_distance = 100.0 * map.slider_mult as f32 / map.tick_rate as f32;

                if map.version >= 8 {
                    tick_distance /=
                        (100.0 / slider_state.speed_mult).max(10.0).min(1000.0) / 100.0;
                }

                let duration = *repeats as f32 * slider_state.beat_len * pixel_len
                    / (map.slider_mult as f32 * slider_state.speed_mult)
                    / 100.0;

                // Build the curve w.r.t. the curve points
                let curve = Curve::new(control_points, pixel_len as f64, &mut curve_bufs);

                let mut current_distance = tick_distance;
                let time_add = duration * (tick_distance / (pixel_len * *repeats as f32));

                let target = pixel_len - tick_distance / 8.0;
                ticks.reserve((target / tick_distance) as usize);

                // Tick of the first span
                if current_distance < target {
                    for tick_idx in 1.. {
                        let pos = curve.position_at((current_distance / pixel_len) as f64);
                        let time = h.start_time as f32 + time_add * tick_idx as f32;
                        ticks.push((pos, time));
                        current_distance += tick_distance;

                        if current_distance >= target {
                            break;
                        }
                    }
                }

                tiny_droplets +=
                    tiny_droplet_count(h.start_time as f32, time_add, duration, *repeats, &ticks);

                let mut slider_objects = Vec::with_capacity(repeats * (ticks.len() + 1));
                slider_objects.push((h.pos, h.start_time as f32));

                // Other spans
                if *repeats <= 1 {
                    slider_objects.append(&mut ticks); // automatically empties buffer for next slider
                } else {
                    slider_objects.extend(&ticks);

                    for repeat_id in 1..*repeats {
                        let time_offset = (duration / *repeats as f32) * repeat_id as f32;
                        let pos = curve.position_at((repeat_id % 2) as f64);

                        // Reverse tick
                        slider_objects.push((pos, h.start_time as f32 + time_offset));

                        // Actual ticks
                        if repeat_id & 1 == 1 {
                            slider_objects.extend(ticks.iter().rev().enumerate().map(
                                |(i, (pos, time))| (*pos, *time + time_add * 2.0 * (i + 1) as f32),
                            ));
                        } else {
                            slider_objects.extend(ticks.iter().copied());
                        }
                    }

                    ticks.clear();
                }

                // Slider tail
                let pos = curve.position_at((*repeats % 2) as f64);
                slider_objects.push((pos, h.start_time as f32 + duration));

                fruits += 1 + *repeats;
                droplets += slider_objects.len() - 1 - *repeats;

                let iter = slider_objects.into_iter().map(CatchObject::new);

                Some(Some(FruitOrJuice::Juice(iter)))
            }
            HitObjectKind::Spinner { .. } | HitObjectKind::Hold { .. } => Some(None),
        })
        .filter_map(identity)
        .flatten()
        .take(take);

    // Hyper dash business
    let base_size = calculate_catch_width(attributes.cs as f32) * 0.5;
    let half_catcher_width = base_size * 0.8;
    let catcher_size = base_size;

    let mut last_direction = 0;
    let mut last_excess = catcher_size;

    // Strain business
    let mut movement = Movement::new();
    let section_len = SECTION_LENGTH * attributes.clock_rate as f32;
    let mut current_section_end =
        (map.hit_objects[0].start_time as f32 / section_len).ceil() * section_len;

    let mut prev = hit_objects.next().unwrap();
    let mut curr = hit_objects.next().unwrap();

    prev.init_hyper_dash(catcher_size, &curr, &mut last_direction, &mut last_excess);

    // Handle second object separately to remove later if-branching
    let next = hit_objects.next().unwrap();
    curr.init_hyper_dash(catcher_size, &next, &mut last_direction, &mut last_excess);

    let h = DifficultyObject::new(
        &curr,
        &prev,
        half_catcher_width,
        attributes.clock_rate as f32,
    );

    while h.base.time as f32 > current_section_end {
        current_section_end += section_len;
    }

    movement.process(&h);

    prev = curr;
    curr = next;

    // Handle all other objects
    for next in hit_objects {
        curr.init_hyper_dash(catcher_size, &next, &mut last_direction, &mut last_excess);

        let h = DifficultyObject::new(
            &curr,
            &prev,
            half_catcher_width,
            attributes.clock_rate as f32,
        );

        while h.base.time as f32 > current_section_end {
            movement.save_current_peak();
            movement.start_new_section_from(current_section_end);
            current_section_end += section_len;
        }

        movement.process(&h);

        prev = curr;
        curr = next;
    }

    // Same as in loop but without init_hyper_dash because `curr` is the last element
    let h = DifficultyObject::new(
        &curr,
        &prev,
        half_catcher_width,
        attributes.clock_rate as f32,
    );

    while h.base.time as f32 > current_section_end {
        movement.save_current_peak();
        movement.start_new_section_from(current_section_end);

        current_section_end += section_len;
    }

    movement.process(&h);
    movement.save_current_peak();

    let stars = movement.difficulty_value().sqrt() * STAR_SCALING_FACTOR;

    FruitsDifficultyAttributes {
        stars: stars as f64,
        ar: attributes.ar,
        n_fruits: fruits,
        n_droplets: droplets,
        n_tiny_droplets: tiny_droplets,
        max_combo: fruits + droplets,
    }
}

// BUG: Sometimes there are off-by-one errors,
// presumably caused by floating point inaccuracies
fn tiny_droplet_count(
    start_time: f32,
    time_between_ticks: f32,
    duration: f32,
    spans: usize,
    ticks: &[(Pos2, f32)],
) -> usize {
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
    let repeat_time = start_time + duration / spans as f32;
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
    let end_time = start_time + duration / spans as f32 - LEGACY_LAST_TICK_OFFSET;
    let since_last_tick = end_time - last;

    let last_section = if since_last_tick > 80.0 {
        let time_between_tiny = shrink_down(since_last_tick);

        count_iterations(time_between_tiny, time_between_tiny, since_last_tick)
    } else {
        0
    };

    // Combine tiny droplets counts
    per_tick * ticks.len() * spans + span_last_section * (spans.saturating_sub(1)) + last_section
}

#[inline]
fn shrink_down(mut val: f32) -> f32 {
    while val > 100.0 {
        val /= 2.0;
    }

    val
}

#[inline]
fn count_iterations(mut start: f32, step: f32, end: f32) -> usize {
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

enum FruitOrJuice<I> {
    Fruit(Option<CatchObject>),
    Juice(I),
}

impl<I: Iterator<Item = CatchObject>> Iterator for FruitOrJuice<I> {
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
