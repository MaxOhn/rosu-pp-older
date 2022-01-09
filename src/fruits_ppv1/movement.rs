use super::DifficultyObject;

use std::cmp::Ordering;

const ABSOLUTE_PLAYER_POSITIONING_ERROR: f32 = 16.0;
const NORMALIZED_HITOBJECT_RADIUS: f32 = 41.0;
const POSITION_EPSILON: f32 = NORMALIZED_HITOBJECT_RADIUS - ABSOLUTE_PLAYER_POSITIONING_ERROR;
const DIRECTION_CHANGE_BONUS: f32 = 12.5;

const SKILL_MULTIPLIER: f64 = 850.0;
const STRAIN_DECAY_BASE: f64 = 0.2;

const DECAY_WEIGHT: f64 = 0.94;

pub(crate) struct Movement {
    last_player_position: Option<f32>,
    last_distance_moved: f32,

    current_strain: f64,
    current_section_peak: f64,

    pub(crate) strain_peaks: Vec<f64>,
    prev_time: Option<f64>,
}

impl Movement {
    #[inline]
    pub(crate) fn new() -> Self {
        Self {
            last_player_position: None,
            last_distance_moved: 0.0,

            current_strain: 1.0,
            current_section_peak: 1.0,

            strain_peaks: Vec::with_capacity(128),
            prev_time: None,
        }
    }

    #[inline]
    pub(crate) fn save_current_peak(&mut self) {
        self.strain_peaks.push(self.current_section_peak);
    }

    #[inline]
    pub(crate) fn start_new_section_from(&mut self, time: f64) {
        self.current_section_peak = self.peak_strain(time - self.prev_time.unwrap());
    }

    pub(crate) fn process(&mut self, current: &DifficultyObject) {
        self.current_strain *= strain_decay(current.delta);
        self.current_strain += self.strain_value_of(current) * SKILL_MULTIPLIER;
        self.current_section_peak = self.current_strain.max(self.current_section_peak);
        self.prev_time.replace(current.base.time);
    }

    pub(crate) fn difficulty_value(&mut self) -> f64 {
        let mut difficulty = 0.0;
        let mut weight = 1.0;

        self.strain_peaks
            .sort_unstable_by(|a, b| b.partial_cmp(a).unwrap_or(Ordering::Equal));

        for &strain in self.strain_peaks.iter() {
            difficulty += strain * weight;
            weight *= DECAY_WEIGHT;
        }

        difficulty
    }

    fn strain_value_of(&mut self, current: &DifficultyObject) -> f64 {
        let last_player_pos = self
            .last_player_position
            .unwrap_or(current.last_normalized_pos);

        let mut pos = last_player_pos
            .max(current.normalized_pos - POSITION_EPSILON)
            .min(current.normalized_pos + POSITION_EPSILON);

        let dist_moved = pos - last_player_pos;

        let mut dist_addition = dist_moved.abs().powf(1.3) / 500.0;
        let sqrt_strain = current.strain_time.sqrt() as f32;

        let mut bonus = 0.0;

        if dist_moved.abs() > 0.1 {
            if self.last_distance_moved.abs() > 0.1
                && dist_moved.signum() != self.last_distance_moved.signum()
            {
                let bonus_factor = dist_moved.abs().min(ABSOLUTE_PLAYER_POSITIONING_ERROR)
                    / ABSOLUTE_PLAYER_POSITIONING_ERROR;

                dist_addition += DIRECTION_CHANGE_BONUS / sqrt_strain * bonus_factor;

                if current.last.hyper_dist <= 10.0 {
                    bonus = 0.3 * bonus_factor;
                }
            }

            dist_addition += 7.5 * dist_moved.abs().min(NORMALIZED_HITOBJECT_RADIUS * 2.0)
                / (NORMALIZED_HITOBJECT_RADIUS * 6.0)
                / sqrt_strain;
        }

        if current.last.hyper_dist <= 10.0 {
            if current.last.hyper_dash {
                pos = current.normalized_pos;
            } else {
                bonus += 1.0;
            }

            dist_addition *= 1.0 + bonus * ((10.0 - current.last.hyper_dist) / 10.0);
        }

        self.last_player_position.replace(pos);
        self.last_distance_moved = dist_moved;

        dist_addition as f64 / current.strain_time
    }

    #[inline]
    fn peak_strain(&self, delta_time: f64) -> f64 {
        self.current_strain * strain_decay(delta_time)
    }
}

#[inline]
fn strain_decay(ms: f64) -> f64 {
    STRAIN_DECAY_BASE.powf(ms / 1000.0)
}
