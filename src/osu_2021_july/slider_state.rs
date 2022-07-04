use rosu_pp::{
    beatmap::{ControlPoint, ControlPointIter},
    Beatmap,
};

pub(crate) struct SliderState<'p> {
    control_points: ControlPointIter<'p>,
    next: Option<ControlPoint>,
    pub(crate) beat_len: f32,
    pub(crate) speed_mult: f32,
}

impl<'p> SliderState<'p> {
    #[inline]
    pub(crate) fn new(map: &'p Beatmap) -> Self {
        let mut control_points = map.control_points();

        let (beat_len, speed_mult) = match control_points.next() {
            Some(ControlPoint::Timing(point)) => (point.beat_len as f32, 1.0),
            Some(ControlPoint::Difficulty(point)) => (1000.0, point.speed_multiplier as f32),
            None => (1000.0, 1.0),
        };

        Self {
            next: control_points.next(),
            control_points,
            beat_len,
            speed_mult,
        }
    }

    #[inline]
    pub(crate) fn update(&mut self, time: f32) {
        while let Some(next) = self.next.as_ref().filter(|n| time >= n.time() as f32) {
            match next {
                ControlPoint::Timing(point) => {
                    self.beat_len = point.beat_len as f32;
                    self.speed_mult = 1.0;
                }
                ControlPoint::Difficulty(point) => self.speed_mult = point.speed_multiplier as f32,
            }

            self.next = self.control_points.next();
        }
    }
}
