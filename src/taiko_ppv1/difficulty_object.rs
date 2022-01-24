use super::rim::Rim;

use rosu_pp::parse::HitObject;

#[derive(Clone, Debug)]
pub(crate) struct DifficultyObject<'o> {
    pub(crate) base: &'o HitObject,
    pub(crate) prev: &'o HitObject,
    pub(crate) delta: f32,
    pub(crate) has_type_change: bool,
}

impl<'o> DifficultyObject<'o> {
    #[inline]
    pub(crate) fn new(
        (base, base_sound): (&'o HitObject, &u8),
        (prev, prev_sound): (&'o HitObject, &u8),
        clock_rate: f32,
    ) -> Self {
        let delta = (base.start_time as f32 - prev.start_time as f32) / clock_rate;
        let has_type_change = prev_sound.is_rim() != base_sound.is_rim();

        Self {
            base,
            prev,
            delta,
            has_type_change,
        }
    }
}
