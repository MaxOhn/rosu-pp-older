use crate::{
    any_2024::difficulty::{
        object::IDifficultyObject,
        skills::{strain_decay, ISkill, Skill, StrainDecaySkill, StrainSkill},
    },
    taiko_2024::{
        difficulty::object::{TaikoDifficultyObject, TaikoDifficultyObjects},
        object::HitType,
    },
    util::{strains_vec::StrainsVec, sync::Weak},
};

const SKILL_MULTIPLIER: f64 = 1.1;
const STRAIN_DECAY_BASE: f64 = 0.4;

#[derive(Clone)]
pub struct Stamina {
    inner: StrainSkill,
    single_color: bool,
    curr_strain: f64,
}

impl Stamina {
    pub fn new(single_color: bool) -> Self {
        Self {
            inner: StrainSkill::default(),
            single_color,
            curr_strain: 0.0,
        }
    }

    pub fn get_curr_strain_peaks(self) -> StrainsVec {
        self.inner.get_curr_strain_peaks().into_strains()
    }

    pub fn as_difficulty_value(&self) -> f64 {
        self.inner
            .clone()
            .difficulty_value(StrainDecaySkill::DECAY_WEIGHT)
            .difficulty_value()
    }
}

impl ISkill for Stamina {
    type DifficultyObjects<'a> = TaikoDifficultyObjects;
}

impl Skill<'_, Stamina> {
    fn calculate_initial_strain(&mut self, time: f64, curr: &TaikoDifficultyObject) -> f64 {
        if self.inner.single_color {
            return 0.0;
        }

        let prev_start_time = curr
            .previous(0, &self.diff_objects.objects)
            .map_or(0.0, |prev| prev.get().start_time);

        self.curr_strain() * strain_decay(time - prev_start_time, STRAIN_DECAY_BASE)
    }

    const fn curr_strain(&self) -> f64 {
        self.inner.curr_strain
    }

    fn curr_strain_mut(&mut self) -> &mut f64 {
        &mut self.inner.curr_strain
    }

    const fn curr_section_peak(&self) -> f64 {
        self.inner.inner.curr_section_peak
    }

    fn curr_section_peak_mut(&mut self) -> &mut f64 {
        &mut self.inner.inner.curr_section_peak
    }

    const fn curr_section_end(&self) -> f64 {
        self.inner.inner.curr_section_end
    }

    fn curr_section_end_mut(&mut self) -> &mut f64 {
        &mut self.inner.inner.curr_section_end
    }

    pub fn process(&mut self, curr: &TaikoDifficultyObject) {
        if curr.idx == 0 {
            *self.curr_section_end_mut() = (curr.start_time / StrainDecaySkill::SECTION_LEN).ceil()
                * StrainDecaySkill::SECTION_LEN;
        }

        while curr.start_time > self.curr_section_end() {
            self.inner.inner.save_curr_peak();
            let initial_strain = self.calculate_initial_strain(self.curr_section_end(), curr);
            self.inner.inner.start_new_section_from(initial_strain);
            *self.curr_section_end_mut() += StrainDecaySkill::SECTION_LEN;
        }

        let strain_value_at = self.strain_value_at(curr);
        *self.curr_section_peak_mut() = strain_value_at.max(self.curr_section_peak());
    }

    fn strain_value_at(&mut self, curr: &TaikoDifficultyObject) -> f64 {
        *self.curr_strain_mut() *= strain_decay(curr.delta_time, STRAIN_DECAY_BASE);
        *self.curr_strain_mut() +=
            StaminaEvaluator::evaluate_diff_of(curr, self.diff_objects) * SKILL_MULTIPLIER;

        // Safely prevents previous strains from shifting as new notes are added.
        let index = curr
            .color
            .mono_streak
            .as_ref()
            .and_then(Weak::upgrade)
            .and_then(|mono| {
                mono.get().hit_objects.iter().position(|h| {
                    let Some(h) = h.upgrade() else { return false };
                    let h = h.get();

                    h.idx == curr.idx
                })
            })
            .unwrap_or(0);

        if self.inner.single_color {
            self.curr_strain() / (1.0 + f64::exp((-(index as isize - 10)) as f64 / 2.0))
        } else {
            self.curr_strain()
        }
    }
}

struct StaminaEvaluator;

impl StaminaEvaluator {
    fn speed_bonus(mut interval: f64) -> f64 {
        // * Interval is capped at a very small value to prevent infinite values.
        interval = interval.max(1.0);

        30.0 / interval
    }

    fn available_fingers_for(
        hit_object: &TaikoDifficultyObject,
        hit_objects: &TaikoDifficultyObjects,
    ) -> usize {
        let prev_color_change = hit_object.color.previous_color_change(hit_objects);

        if prev_color_change
            .is_some_and(|change| hit_object.start_time - change.get().start_time < 300.0)
        {
            return 2;
        }

        let next_color_change = hit_object.color.next_color_change(hit_objects);

        if next_color_change
            .is_some_and(|change| change.get().start_time - hit_object.start_time < 300.0)
        {
            return 2;
        }

        4
    }

    fn evaluate_diff_of(curr: &TaikoDifficultyObject, hit_objects: &TaikoDifficultyObjects) -> f64 {
        if matches!(curr.base_hit_type, HitType::NonHit) {
            return 0.0;
        }

        // * Find the previous hit object hit by the current finger, which is n notes prior, n being the number of
        // * available fingers.
        let taiko_curr = curr;
        let key_prev = hit_objects.previous_mono(
            taiko_curr,
            Self::available_fingers_for(taiko_curr, hit_objects) - 1,
        );

        if let Some(key_prev) = key_prev {
            // * Add a base strain to all objects
            0.5 + Self::speed_bonus(taiko_curr.start_time - key_prev.get().start_time)
        } else {
            // * There is no previous hit object hit by the current finger
            0.0
        }
    }
}
