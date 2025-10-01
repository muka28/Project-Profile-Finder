use super::models::Profile;

pub trait ProfileMatcher {
    fn score(&self, actual: &Profile, target: &Profile) -> f64;
}

pub struct AreaMatcher {
    pub use_offset: bool,
}

impl ProfileMatcher for AreaMatcher {
    fn score(&self, actual: &Profile, target: &Profile) -> f64 {
        let l = target.total_length();
        if l == 0.0 {
            return 0.0;
        }
        // To compute integral |actual(s) - target(s) - z0| ds
        // First, compute without offset
        let mut area = 0.0;
        let mut i_a = 0;
        let mut i_t = 0;
        let mut s = 0.0;
        while i_a < actual.points.len() - 1 || i_t < target.points.len() - 1 {
            let next_s_a = if i_a < actual.points.len() - 1 { actual.points[i_a + 1].0 } else { f64::MAX };
            let next_s_t = if i_t < target.points.len() - 1 { target.points[i_t + 1].0 } else { f64::MAX };
            let next_s = next_s_a.min(next_s_t);
            let len = next_s - s;
            if len > 0.0 {
                let a_start = actual.interpolate(s);
                let a_end = actual.interpolate(next_s);
                let t_start = target.interpolate(s);
                let t_end = target.interpolate(next_s);
                area += integral_abs_diff(len, a_start - t_start, a_end - t_end);
            }
            s = next_s;
            if next_s == next_s_a {
                i_a += 1;
            }
            if next_s == next_s_t {
                i_t += 1;
            }
        }
        if !self.use_offset {
            return area;
        }
        // Simple offset using average at sample points
        let samples = &target.points;
        let mut sum_diff = 0.0;
        for p in samples {
            sum_diff += actual.interpolate(p.0) - p.1;
        }
        let z0 = -sum_diff / samples.len() as f64;
        // Recompute area with z0
        let mut area_offset = 0.0;
        // Similar loop, but add z0 to actual (or subtract from diff)
        let mut i_a = 0;
        let mut i_t = 0;
        let mut s = 0.0;
        while i_a < actual.points.len() - 1 || i_t < target.points.len() - 1 {
            let next_s_a = if i_a < actual.points.len() - 1 { actual.points[i_a + 1].0 } else { f64::MAX };
            let next_s_t = if i_t < target.points.len() - 1 { target.points[i_t + 1].0 } else { f64::MAX };
            let next_s = next_s_a.min(next_s_t);
            let len = next_s - s;
            if len > 0.0 {
                let a_start = actual.interpolate(s) + z0;
                let a_end = actual.interpolate(next_s) + z0;
                let t_start = target.interpolate(s);
                let t_end = target.interpolate(next_s);
                area_offset += integral_abs_diff(len, a_start - t_start, a_end - t_end);
            }
            s = next_s;
            if next_s == next_s_a {
                i_a += 1;
            }
            if next_s == next_s_t {
                i_t += 1;
            }
        }
        area_offset
    }
}

// Integral of |diff_start + t/len * (diff_end - diff_start)| dt over [0, len]
pub fn integral_abs_diff(len: f64, diff_start: f64, diff_end: f64) -> f64 {
    if diff_start.signum() == diff_end.signum() || len == 0.0 {
        return (diff_start.abs() + diff_end.abs()) / 2.0 * len;
    }
    // Crosses zero
    let t0 = -diff_start / (diff_end - diff_start) * len;
    let area1 = diff_start.abs() * t0 / 2.0;  // Triangle
    let area2 = diff_end.abs() * (len - t0) / 2.0;
    area1 + area2
}