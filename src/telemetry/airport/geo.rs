//! Small-scale navigation maths (nautical miles, true degrees) for airport traffic.

const EARTH_RADIUS_NM: f64 = 3440.065;

/// Great-circle distance between two points, in nautical miles.
pub fn distance_nm(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let (p1, p2) = (lat1.to_radians(), lat2.to_radians());
    let dp = (lat2 - lat1).to_radians();
    let dl = (lon2 - lon1).to_radians();
    let a = (dp / 2.0).sin().powi(2) + p1.cos() * p2.cos() * (dl / 2.0).sin().powi(2);
    2.0 * EARTH_RADIUS_NM * a.sqrt().asin()
}

/// Initial true bearing from point 1 to point 2 (`0.0..360.0`).
pub fn bearing_deg(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let (p1, p2) = (lat1.to_radians(), lat2.to_radians());
    let dl = (lon2 - lon1).to_radians();
    let y = dl.sin() * p2.cos();
    let x = p1.cos() * p2.sin() - p1.sin() * p2.cos() * dl.cos();
    y.atan2(x).to_degrees().rem_euclid(360.0)
}

/// Smallest absolute difference between two headings (`0.0..=180.0`).
pub fn angle_diff_deg(a: f64, b: f64) -> f64 {
    let d = (a - b).rem_euclid(360.0);
    d.min(360.0 - d)
}

/// Perpendicular distance (nm) from point `p` to the infinite line through `a` and `b`,
/// using a flat projection centred on `a` (accurate within a few tens of miles).
pub fn cross_track_nm(p: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    let k = a.0.to_radians().cos();
    let to_xy = |q: (f64, f64)| ((q.1 - a.1) * k * 60.0, (q.0 - a.0) * 60.0);
    let (bx, by) = to_xy(b);
    let (px, py) = to_xy(p);
    let len = (bx * bx + by * by).sqrt();
    if len < 1e-9 {
        return (px * px + py * py).sqrt();
    }
    (bx * py - by * px).abs() / len
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_distance_and_bearing_lcy_to_lhr() {
        // London City (EGLC) to Heathrow (EGLL) is roughly 19 nm, a little south of due west.
        let d = distance_nm(51.5053, 0.0553, 51.4706, -0.4619);
        assert!((d - 19.4).abs() < 0.5, "distance was {d}");
        let b = bearing_deg(51.5053, 0.0553, 51.4706, -0.4619);
        assert!((b - 263.0).abs() < 2.0, "bearing was {b}");
    }

    #[test]
    fn test_angle_diff_wraps() {
        assert_eq!(angle_diff_deg(350.0, 10.0), 20.0);
        assert_eq!(angle_diff_deg(93.0, 273.0), 180.0);
        assert_eq!(angle_diff_deg(273.0, 270.0), 3.0);
    }

    #[test]
    fn test_cross_track_offset() {
        // EGLC runway 09/27 thresholds; a point 1 nm north of the centreline.
        let le = (51.505576, 0.044333);
        let he = (51.504894, 0.066026);
        let p = (51.505576 + 1.0 / 60.0, 0.03);
        let off = cross_track_nm(p, le, he);
        assert!((off - 1.0).abs() < 0.05, "offset was {off}");
    }
}
