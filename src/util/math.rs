pub fn lerp(start: f64, end: f64, percent: f64) -> f64 {
    start + (end - start) * percent
}

pub fn difficulty_range(val: f64, max: f64, avg: f64, min: f64) -> f64 {
    if val > 5.0 {
        avg + (max - avg) * (val - 5.0) / 5.0
    } else if val < 5.0 {
        avg - (avg - min) * (5.0 - val) / 5.0
    } else {
        avg
    }
}
