use std::{borrow::Cow, cmp::Ordering, convert::identity, f32::consts::PI, iter};

use rosu_pp::parse::{PathControlPoint, PathType, Pos2};

const BEZIER_TOLERANCE: f32 = 0.25;
const CATMULL_DETAIL: usize = 50;
const CIRCULAR_ARC_TOLERANCE: f32 = 0.1;

#[derive(Default)]
pub(crate) struct CurveBuffers {
    vertices: Vec<Pos2>,
    bezier: BezierBuffers,
}

#[derive(Default)]
struct BezierBuffers {
    buf1: Vec<Pos2>,
    buf2: Vec<Pos2>,
    buf3: Vec<Pos2>,
    buf4: Vec<Pos2>,
}

impl BezierBuffers {
    /// Fill the buffers with new elements until a
    /// length of `len` is reached. Does nothing if `len`
    /// is already smaller than the current buffer size.
    fn extend_exact(&mut self, len: usize) {
        if len <= self.buf1.len() {
            return;
        }

        let additional = len - self.buf1.len();

        self.buf1
            .extend(iter::repeat(Pos2::zero()).take(additional));
        self.buf2
            .extend(iter::repeat(Pos2::zero()).take(additional));
        self.buf3
            .extend(iter::repeat(Pos2::zero()).take(additional));
        self.buf4
            .extend(iter::repeat(Pos2::zero()).take(additional));
    }
}

struct CircularArcProperties {
    theta_start: f64,
    theta_range: f64,
    direction: f64,
    radius: f32,
    centre: Pos2,
}

pub(crate) struct Curve {
    path: Vec<Pos2>,
    lengths: Vec<f64>,
}

impl Curve {
    pub(crate) fn new(
        points: &[PathControlPoint],
        expected_len: f64,
        bufs: &mut CurveBuffers,
    ) -> Self {
        let mut path = Self::calculate_path(points, bufs);
        let lengths = Self::calculate_length(points, &mut path, expected_len);

        Self { path, lengths }
    }

    pub(crate) fn position_at(&self, progress: f64) -> Pos2 {
        let d = self.progress_to_dist(progress);
        let i = self.idx_of_dist(d);

        self.interpolate_vertices(i, d)
    }

    fn progress_to_dist(&self, progress: f64) -> f64 {
        progress.clamp(0.0, 1.0) * self.dist()
    }

    pub(crate) fn dist(&self) -> f64 {
        self.lengths.last().copied().unwrap_or(0.0)
    }

    fn idx_of_dist(&self, d: f64) -> usize {
        self.lengths
            .binary_search_by(|len| len.partial_cmp(&d).unwrap_or(Ordering::Equal))
            .map_or_else(identity, identity)
    }

    fn interpolate_vertices(&self, i: usize, d: f64) -> Pos2 {
        if self.path.is_empty() {
            return Pos2::zero();
        }

        let p1 = if i == 0 {
            return self.path[0];
        } else if let Some(p) = self.path.get(i) {
            *p
        } else {
            return self.path[self.path.len() - 1];
        };

        let p0 = self.path[i - 1];

        let d0 = self.lengths[i - 1];
        let d1 = self.lengths[i];

        // * Avoid division by an almost-zero number in case
        // * two points are extremely close to each other
        if (d0 - d1).abs() <= f64::EPSILON {
            return p0;
        }

        let w = (d - d0) / (d1 - d0);

        p0 + (p1 - p0) * w as f32
    }

    fn calculate_path(points: &[PathControlPoint], bufs: &mut CurveBuffers) -> Vec<Pos2> {
        if points.is_empty() {
            return Vec::new();
        }

        let CurveBuffers { vertices, bezier } = bufs;

        vertices.clear();
        vertices.extend(points.iter().map(|p| p.pos));

        let mut path = Vec::new();
        let mut start = 0;

        for i in 0..points.len() {
            if points[i].kind.is_none() && i < points.len() - 1 {
                continue;
            }

            // * The current vertex ends the segment
            let segment_vertices = &vertices[start..i + 1];
            let segment_kind = points[start].kind.unwrap_or(PathType::Linear);

            Self::calculate_subpath(&mut path, segment_vertices, segment_kind, bezier);

            // * Start the new segment at the current vertex
            start = i;
        }

        path.dedup();

        path
    }

    fn calculate_length(
        points: &[PathControlPoint],
        path: &mut Vec<Pos2>,
        expected_len: f64,
    ) -> Vec<f64> {
        let mut calculated_len = 0.0;
        let mut cumulative_len = Vec::with_capacity(path.len());
        cumulative_len.push(0.0);

        let length_iter = path.iter().zip(path.iter().skip(1)).map(|(&curr, &next)| {
            calculated_len += (next - curr).length() as f64;

            calculated_len
        });

        cumulative_len.extend(length_iter);

        if (expected_len - calculated_len).abs() > f64::EPSILON {
            // * In osu-stable, if the last two control points of a slider are equal, extension is not performed
            let condition_opt = points
                .len()
                .checked_sub(2)
                .and_then(|i| points.get(i..))
                .filter(|suffix| suffix[0].pos == suffix[1].pos && expected_len > calculated_len);

            if condition_opt.is_some() {
                cumulative_len.push(calculated_len);

                return cumulative_len;
            }

            // * The last length is always incorrect
            cumulative_len.pop();

            let last_valid = cumulative_len
                .iter()
                .rev()
                .position(|l| *l < expected_len)
                .map_or(0, |idx| cumulative_len.len() - idx);

            // * The path will be shortened further, in which case we should trim
            // * any more unnecessary lengths and their associated path segments
            if last_valid < cumulative_len.len() {
                cumulative_len.truncate(last_valid);
                path.truncate(last_valid + 1);

                if cumulative_len.is_empty() {
                    // * The expected distance is negative or zero
                    // * Perhaps negative path lengths should be disallowed altogether
                    cumulative_len.push(0.0);

                    return cumulative_len;
                }
            }

            let end_idx = cumulative_len.len();
            let prev_idx = end_idx - 1;

            // * The direction of the segment to shorten or lengthen
            let dir = (path[end_idx] - path[prev_idx]).normalize();

            path[end_idx] = path[prev_idx] + dir * (expected_len - cumulative_len[prev_idx]) as f32;
            cumulative_len.push(expected_len);
        }

        cumulative_len
    }

    fn calculate_subpath(
        path: &mut Vec<Pos2>,
        sub_points: &[Pos2],
        kind: PathType,
        bufs: &mut BezierBuffers,
    ) {
        match kind {
            PathType::Bezier => Self::approximate_bezier(path, sub_points, bufs),
            PathType::Catmull => Self::approximate_catmull(path, sub_points),
            PathType::Linear => Self::approximate_linear(path, sub_points),
            PathType::PerfectCurve => {
                if let [a, b, c] = sub_points {
                    if Self::approximate_circular_arc(path, *a, *b, *c) {
                        return;
                    }
                }

                Self::approximate_bezier(path, sub_points, bufs)
            }
        }
    }

    fn approximate_bezier(path: &mut Vec<Pos2>, points: &[Pos2], bufs: &mut BezierBuffers) {
        bufs.extend_exact(points.len());

        Self::approximate_bspline(path, points, bufs);
    }

    fn approximate_catmull(path: &mut Vec<Pos2>, points: &[Pos2]) {
        if points.len() == 1 {
            return;
        }

        path.reserve_exact((points.len() - 1) * CATMULL_DETAIL * 2);

        // Handle first iteration distinctly because of v1
        let v1 = points[0];
        let v2 = points[0];
        let v3 = points.get(1).copied().unwrap_or(v2);
        let v4 = points.get(2).copied().unwrap_or_else(|| v3 * 2.0 - v2);

        Self::catmull_subpath(path, v1, v2, v3, v4);

        // Remaining iterations
        for (i, (&v1, &v2)) in (2..points.len()).zip(points.iter().zip(points.iter().skip(1))) {
            let v3 = points.get(i).copied().unwrap_or_else(|| v2 * 2.0 - v1);
            let v4 = points.get(i + 1).copied().unwrap_or_else(|| v3 * 2.0 - v2);

            Self::catmull_subpath(path, v1, v2, v3, v4);
        }
    }

    fn approximate_linear(path: &mut Vec<Pos2>, points: &[Pos2]) {
        path.extend(points)
    }

    fn approximate_circular_arc(path: &mut Vec<Pos2>, a: Pos2, b: Pos2, c: Pos2) -> bool {
        let pr = match Self::circular_arc_properties(a, b, c) {
            Some(pr) => pr,
            None => return false,
        };

        // * We select the amount of points for the approximation by requiring the discrete curvature
        // * to be smaller than the provided tolerance. The exact angle required to meet the tolerance
        // * is: 2 * Math.Acos(1 - TOLERANCE / r)
        // * The special case is required for extremely short sliders where the radius is smaller than
        // * the tolerance. This is a pathological rather than a realistic case.
        let amount_points = if 2.0 * pr.radius <= CIRCULAR_ARC_TOLERANCE {
            2
        } else {
            let divisor = 2.0 * (1.0 - CIRCULAR_ARC_TOLERANCE / pr.radius).acos();

            ((pr.theta_range / divisor as f64).ceil() as usize).max(2)
        };

        path.reserve_exact(amount_points);
        let divisor = (amount_points - 1) as f64;
        let directed_range = pr.direction * pr.theta_range;

        let subpath = (0..amount_points).map(|i| {
            let fract = i as f64 / divisor;
            let theta = pr.theta_start + fract * directed_range;
            let (sin, cos) = theta.sin_cos();

            let origin = Pos2 {
                x: cos as f32,
                y: sin as f32,
            };

            pr.centre + origin * pr.radius
        });

        path.extend(subpath);

        true
    }

    fn approximate_bspline(path: &mut Vec<Pos2>, points: &[Pos2], bufs: &mut BezierBuffers) {
        let p = points.len();

        let mut to_flatten = Vec::new();
        let mut free_bufs = Vec::new();

        // In osu!lazer's code, `p` is always 0 so the first big `if` can be omitted

        to_flatten.push(Cow::Borrowed(points));

        // * "toFlatten" contains all the curves which are not yet approximated well enough.
        // * We use a stack to emulate recursion without the risk of running into a stack overflow.
        // * (More specifically, we iteratively and adaptively refine our curve with a
        // * <a href="https://en.wikipedia.org/wiki/Depth-first_search">Depth-first search</a>
        // * over the tree resulting from the subdivisions we make.)

        // bufs.buf4 will serve as left_child

        while let Some(mut parent) = to_flatten.pop() {
            if Self::bezier_is_flat_enough(&parent) {
                // * If the control points we currently operate on are sufficiently "flat", we use
                // * an extension to De Casteljau's algorithm to obtain a piecewise-linear approximation
                // * of the bezier curve represented by our control points, consisting of the same amount
                // * of points as there are control points.
                Self::bezier_approximate(&parent, path, bufs);
                free_bufs.push(parent);

                continue;
            }

            // * If we do not yet have a sufficiently "flat" (in other words, detailed) approximation we keep
            // * subdividing the curve we are currently operating on.
            let mut right_child = free_bufs
                .pop()
                .unwrap_or_else(|| Cow::Owned(vec![Pos2::zero(); p]));

            Self::bezier_subdivide(
                &parent,
                &mut bufs.buf4,
                right_child.to_mut(),
                &mut bufs.buf1,
            );

            // * We re-use the buffer of the parent for one of the children, so that we save one allocation per iteration.
            parent.to_mut().copy_from_slice(&bufs.buf4[..p]);

            to_flatten.push(right_child);
            to_flatten.push(parent);
        }

        path.push(points[p - 1]);
    }

    fn bezier_is_flat_enough(points: &[Pos2]) -> bool {
        let limit = BEZIER_TOLERANCE * BEZIER_TOLERANCE * 4.0;

        !points
            .iter()
            .zip(points.iter().skip(1))
            .zip(points.iter().skip(2))
            .any(|((&prev, &curr), &next)| (prev - curr * 2.0 + next).length_squared() > limit)
    }

    fn bezier_subdivide(points: &[Pos2], l: &mut [Pos2], r: &mut [Pos2], midpoints: &mut [Pos2]) {
        let count = points.len();
        midpoints[..count].copy_from_slice(&points[..count]);

        for i in (1..count).rev() {
            l[count - i - 1] = midpoints[0];
            r[i] = midpoints[i];

            for j in 0..i {
                midpoints[j] = (midpoints[j] + midpoints[j + 1]) / 2.0;
            }
        }

        l[count - 1] = midpoints[0];
        r[0] = midpoints[0];
    }

    // * https://en.wikipedia.org/wiki/De_Casteljau%27s_algorithm
    fn bezier_approximate(points: &[Pos2], path: &mut Vec<Pos2>, bufs: &mut BezierBuffers) {
        let count = points.len();

        let BezierBuffers {
            buf1: l,
            buf2: r,
            buf3: midpoints,
            ..
        } = bufs;

        Self::bezier_subdivide(points, l, r, midpoints);
        path.push(points[0]);

        let l = &l[..count];
        let r = &r[1..count];

        let subpath = l
            .iter()
            .chain(r)
            .skip(1)
            .zip(l.iter().chain(r).skip(2))
            .zip(l.iter().chain(r).skip(3))
            .step_by(2)
            .map(|((&prev, &curr), &next)| (prev + curr * 2.0 + next) * 0.25);

        path.extend(subpath);
    }

    fn catmull_subpath(path: &mut Vec<Pos2>, v1: Pos2, v2: Pos2, v3: Pos2, v4: Pos2) {
        let x1 = 2.0 * v2.x;
        let x2 = -v1.x + v3.x;
        let x3 = 2.0 * v1.x - 5.0 * v2.x + 4.0 * v3.x - v4.x;
        let x4 = -v1.x + 3.0 * (v2.x - v3.x) + v4.x;

        let y1 = 2.0 * v2.y;
        let y2 = -v1.y + v3.y;
        let y3 = 2.0 * v1.y - 5.0 * v2.y + 4.0 * v3.y - v4.y;
        let y4 = -v1.y + 3.0 * (v2.y - v3.y) + v4.y;

        let catmull_detail = CATMULL_DETAIL as f32;

        let subpath = (0..CATMULL_DETAIL)
            .map(|c| {
                let c = c as f32;
                let t1 = c / catmull_detail;
                let t2 = t1 * t1;
                let t3 = t2 * t1;

                let pos1 = Pos2 {
                    x: 0.5 * (x1 + x2 * t1 + x3 * t2 + x4 * t3),
                    y: 0.5 * (y1 + y2 * t1 + y3 * t2 + y4 * t3),
                };

                let t1 = (c + 1.0) / catmull_detail;
                let t2 = t1 * t1;
                let t3 = t2 * t1;

                let pos2 = Pos2 {
                    x: 0.5 * (x1 + x2 * t1 + x3 * t2 + x4 * t3),
                    y: 0.5 * (y1 + y2 * t1 + y3 * t2 + y4 * t3),
                };

                iter::once(pos1).chain(iter::once(pos2))
            })
            .flatten();

        path.extend(subpath);
    }

    fn circular_arc_properties(a: Pos2, b: Pos2, c: Pos2) -> Option<CircularArcProperties> {
        // * If we have a degenerate triangle where a side-length is almost zero,
        // * then give up and fallback to a more numerically stable method.
        if ((b.y - a.y) * (c.x - a.x) - (b.x - a.x) * (c.y - a.y)).abs() <= f32::EPSILON {
            return None;
        }

        // * See: https://en.wikipedia.org/wiki/Circumscribed_circle#Cartesian_coordinates_2
        let d = 2.0 * (a.x * (b - c).y + b.x * (c - a).y + c.x * (a - b).y);
        let a_sq = a.length_squared();
        let b_sq = b.length_squared();
        let c_sq = c.length_squared();

        let centre = Pos2 {
            x: (a_sq * (b - c).y + b_sq * (c - a).y + c_sq * (a - b).y) / d,
            y: (a_sq * (c - b).x + b_sq * (a - c).x + c_sq * (b - a).x) / d,
        };

        let d_a = a - centre;
        let d_c = c - centre;

        let radius = d_a.length();

        let theta_start = d_a.y.atan2(d_a.x);
        let mut theta_end = d_c.y.atan2(d_c.x);

        while theta_end < theta_start {
            theta_end += 2.0 * PI;
        }

        let mut direction = 1.0;
        let mut theta_range = theta_end - theta_start;

        // * Decide in which direction to draw the circle,
        // * depending on which side of AC B lies.
        let mut ortho_a_to_c = c - a;

        ortho_a_to_c = Pos2 {
            x: ortho_a_to_c.y,
            y: -ortho_a_to_c.x,
        };

        if ortho_a_to_c.dot(b - a) < 0.0 {
            direction = -direction;
            theta_range = 2.0 * PI - theta_range;
        }

        Some(CircularArcProperties {
            theta_start: theta_start as f64,
            theta_range: theta_range as f64,
            direction,
            radius,
            centre,
        })
    }
}
