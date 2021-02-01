use super::OsuObject;

pub(crate) struct DifficultyObject<'h> {
    pub(crate) base: &'h OsuObject,
    pub(crate) dist: f32,
    pub(crate) delta: f32,
}

impl<'h> DifficultyObject<'h> {
    pub(crate) fn new(
        base: &'h OsuObject,
        prev: &OsuObject,
        clock_rate: f32,
        scaling_factor: f32,
    ) -> Self {
        let delta = (base.time - prev.time) / clock_rate;

        let pos = base.pos;
        let travel_dist = prev.travel_dist.unwrap_or(0.0);
        let prev_cursor_pos = prev.end_pos;

        let dist = (travel_dist + (pos - prev_cursor_pos).length()) * scaling_factor;

        Self { base, dist, delta }
    }
}
