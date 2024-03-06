use rosu_pp::model::control_point::{DifficultyPoint, TimingPoint};

pub fn timing_point_at(points: &[TimingPoint], time: f64) -> Option<&TimingPoint> {
    let i = points
        .binary_search_by(|probe| probe.time.total_cmp(&time))
        .unwrap_or_else(|i| i.saturating_sub(1));

    points.get(i)
}

pub fn difficulty_point_at(points: &[DifficultyPoint], time: f64) -> Option<&DifficultyPoint> {
    points
        .binary_search_by(|probe| probe.time.total_cmp(&time))
        .map_or_else(|i| i.checked_sub(1), Some)
        .map(|i| &points[i])
}
