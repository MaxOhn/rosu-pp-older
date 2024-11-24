use rosu_map::section::hit_objects::{BorrowedCurve, CurveBuffers};
use rosu_pp::{
    model::{
        control_point::{DifficultyPoint, TimingPoint},
        hit_object::{HitObject, HitObjectKind, HoldNote, Spinner},
        mode::GameMode,
    },
    Beatmap,
};

use crate::util::control_points::{difficulty_point_at, timing_point_at};

pub struct ManiaObject {
    pub start_time: f64,
    pub end_time: f64,
    pub column: usize,
}

impl ManiaObject {
    pub fn new(h: &HitObject, total_columns: f32, params: &mut ObjectParams<'_>) -> Self {
        let column = Self::column(h.pos.x, total_columns);
        params.max_combo += 1;

        match h.kind {
            HitObjectKind::Circle => Self {
                start_time: h.start_time,
                end_time: h.start_time,
                column,
            },
            HitObjectKind::Slider(ref slider) => {
                const BASE_SCORING_DIST: f32 = 100.0;

                let dist = BorrowedCurve::new(
                    GameMode::Mania,
                    &slider.control_points,
                    slider.expected_dist,
                    &mut params.curve_bufs,
                )
                .dist();

                let beat_len = timing_point_at(&params.map.timing_points, h.start_time)
                    .map_or(TimingPoint::DEFAULT_BEAT_LEN, |point| point.beat_len);

                let slider_velocity =
                    difficulty_point_at(&params.map.difficulty_points, h.start_time)
                        .map_or(DifficultyPoint::DEFAULT_SLIDER_VELOCITY, |point| {
                            point.slider_velocity
                        });

                let scoring_dist =
                    f64::from(BASE_SCORING_DIST) * params.map.slider_multiplier * slider_velocity;
                let velocity = scoring_dist / beat_len;

                let duration = (slider.span_count() as f64) * dist / velocity;

                params.max_combo += (duration / 100.0) as u32;

                Self {
                    start_time: h.start_time,
                    end_time: h.start_time + duration,
                    column,
                }
            }
            HitObjectKind::Spinner(Spinner { duration })
            | HitObjectKind::Hold(HoldNote { duration }) => {
                params.max_combo += (duration / 100.0) as u32;

                Self {
                    start_time: h.start_time,
                    end_time: h.start_time + duration,
                    column,
                }
            }
        }
    }

    pub fn column(x: f32, total_columns: f32) -> usize {
        let x_divisor = 512.0 / total_columns;

        (x / x_divisor).floor().min(total_columns - 1.0) as usize
    }
}

pub struct ObjectParams<'a> {
    map: &'a Beatmap,
    max_combo: u32,
    curve_bufs: CurveBuffers,
}

impl<'a> ObjectParams<'a> {
    pub fn new(map: &'a Beatmap) -> Self {
        Self {
            map,
            max_combo: 0,
            curve_bufs: CurveBuffers::default(),
        }
    }

    pub fn into_max_combo(self) -> u32 {
        self.max_combo
    }
}
