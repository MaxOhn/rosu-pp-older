use super::DifficultyObject;

const SINGLE_SPACING_TRESHOLD: f32 = 125.0;
const STREAM_SPACING_TRESHOLD: f32 = 110.0;
const ALMOST_DIAMETER: f32 = 90.0;

#[derive(Copy, Clone)]
pub(crate) enum SkillKind {
    Aim,
    Speed,
}

impl SkillKind {
    pub(crate) fn strain_value_of(self, current: &DifficultyObject) -> f32 {
        match self {
            Self::Aim => {
                let aim_value = apply_diminishing_exp(current.dist)
                    + (current.travel_dist > 0.0) as u8 as f32
                        * apply_diminishing_exp(current.travel_dist);

                aim_value / current.delta
            }
            Self::Speed => {
                let dist = current.dist + current.travel_dist;

                let speed_value = if dist > SINGLE_SPACING_TRESHOLD {
                    2.5
                } else if dist > STREAM_SPACING_TRESHOLD {
                    1.6 + 0.9 * (dist - STREAM_SPACING_TRESHOLD)
                        / (SINGLE_SPACING_TRESHOLD - STREAM_SPACING_TRESHOLD)
                } else if dist > ALMOST_DIAMETER {
                    1.2 + 0.4 * (dist - ALMOST_DIAMETER)
                        / (STREAM_SPACING_TRESHOLD - ALMOST_DIAMETER)
                } else if dist > ALMOST_DIAMETER / 2.0 {
                    0.95 + 0.25 * (dist - ALMOST_DIAMETER / 2.0) / (ALMOST_DIAMETER / 2.0)
                } else {
                    0.95
                };

                speed_value / current.delta
            }
        }
    }
}

#[inline]
fn apply_diminishing_exp(val: f32) -> f32 {
    val.powf(0.99)
}
