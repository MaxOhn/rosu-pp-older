use super::DifficultyObject;

use std::cmp::Ordering;

const RHYTHM_CHANGE_BASE_THRESHOLD: f32 = 0.2;
const RHYTHM_CHANGE_BASE: f32 = 2.0;

const SKILL_MULTIPLIER: f32 = 1.0;
const STRAIN_DECAY_BASE: f32 = 0.3;

const DECAY_WEIGHT: f32 = 0.9;

pub(crate) struct Strain {
    current_strain: f32,
    current_section_peak: f32,

    same_color_count: usize,
    last_color_switch: ColorSwitch,

    pub(crate) strain_peaks: Vec<f32>,

    prev_delta: Option<f32>,
}

impl Strain {
    #[inline]
    pub(crate) fn new() -> Self {
        Self {
            current_strain: 1.0,
            current_section_peak: 1.0,

            same_color_count: 1,
            last_color_switch: ColorSwitch::None,

            strain_peaks: Vec::with_capacity(128),

            prev_delta: None,
        }
    }

    #[inline]
    pub(crate) fn save_current_peak(&mut self) {
        self.strain_peaks.push(self.current_section_peak);
    }

    #[inline]
    pub(crate) fn start_new_section_from(&mut self, time: f32) {
        self.current_section_peak = self.peak_strain(time - self.prev_delta.unwrap());
    }

    #[inline]
    fn peak_strain(&self, delta_time: f32) -> f32 {
        self.current_strain * self.strain_decay(delta_time)
    }

    #[inline]
    fn strain_decay(&self, ms: f32) -> f32 {
        STRAIN_DECAY_BASE.powf(ms / 1000.0)
    }

    #[inline]
    pub(crate) fn process(&mut self, current: &DifficultyObject) {
        self.current_strain *= self.strain_decay(current.delta);
        self.current_strain += self.strain_value_of(current) * SKILL_MULTIPLIER;
        self.current_section_peak = self.current_strain.max(self.current_section_peak);
        self.prev_delta.replace(current.delta);
    }

    fn strain_value_of(&mut self, current: &DifficultyObject) -> f32 {
        let mut addition = 1.0;

        if current.base.is_circle() && current.prev.is_circle() && current.delta < 1000.0 {
            addition += self.has_color_change(current) as u8 as f32 * 0.75;
            addition += self.has_rhythm_change(current) as u8 as f32;
        } else {
            self.last_color_switch = ColorSwitch::None;
            self.same_color_count = 1;
        }

        let addition_factor = if current.delta < 50.0 {
            0.4 + 0.6 * current.delta / 50.0
        } else {
            1.0
        };

        addition_factor * addition
    }

    fn has_rhythm_change(&mut self, current: &DifficultyObject) -> bool {
        if current.delta.abs() < f32::EPSILON
            || self
                .prev_delta
                .map_or(true, |time| time.abs() < f32::EPSILON)
        {
            return false;
        }

        let prev_time = self.prev_delta.unwrap();

        let time_elapsed_ratio = (prev_time / current.delta).max(current.delta / prev_time);

        if time_elapsed_ratio >= 8.0 {
            return false;
        }

        let difference = (time_elapsed_ratio).log(RHYTHM_CHANGE_BASE) % 1.0;

        difference > RHYTHM_CHANGE_BASE_THRESHOLD && difference < 1.0 - RHYTHM_CHANGE_BASE_THRESHOLD
    }

    fn has_color_change(&mut self, current: &DifficultyObject) -> bool {
        if !current.has_type_change {
            self.same_color_count += 1;

            return false;
        }

        let old_color_switch = self.last_color_switch;

        let new_color_switch = if self.same_color_count % 2 == 0 {
            ColorSwitch::Even
        } else {
            ColorSwitch::Odd
        };

        self.last_color_switch = new_color_switch;
        self.same_color_count = 1;

        old_color_switch != ColorSwitch::None && old_color_switch != new_color_switch
    }

    #[inline]
    pub(crate) fn difficulty_value(&mut self) -> f32 {
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
}

#[derive(Copy, Clone, Eq, PartialEq)]
enum ColorSwitch {
    None,
    Even,
    Odd,
}
