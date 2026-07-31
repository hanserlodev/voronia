pub fn simplify(points: &[[f32; 2]], tolerance: f32) -> Vec<[f32; 2]> {
    if points.len() <= 2 || tolerance <= 0.0 {
        return points.to_vec();
    }
    let sq_tolerance = tolerance * tolerance;
    let points = radial_distance(points, sq_tolerance);
    if points.len() <= 2 {
        return points;
    }
    rdp(&points, 0, points.len() - 1, sq_tolerance)
}

fn get_square_segment_distance(p: &[f32; 2], p1: &[f32; 2], p2: &[f32; 2]) -> f32 {
    let dx = p2[0] - p1[0];
    let dy = p2[1] - p1[1];
    let len2 = dx * dx + dy * dy;
    if len2 == 0.0 {
        return (p[0] - p1[0]) * (p[0] - p1[0]) + (p[1] - p1[1]) * (p[1] - p1[1]);
    }
    let t = ((p[0] - p1[0]) * dx + (p[1] - p1[1]) * dy) / len2;
    let t = t.clamp(0.0, 1.0);
    let proj_x = p1[0] + t * dx;
    let proj_y = p1[1] + t * dy;
    (p[0] - proj_x) * (p[0] - proj_x) + (p[1] - proj_y) * (p[1] - proj_y)
}

fn radial_distance(points: &[[f32; 2]], sq_tolerance: f32) -> Vec<[f32; 2]> {
    if points.is_empty() {
        return points.to_vec();
    }
    let mut result = Vec::with_capacity(points.len());
    result.push(points[0]);
    let mut prev = &points[0];
    for pt in points.iter().skip(1) {
        let dx = pt[0] - prev[0];
        let dy = pt[1] - prev[1];
        if dx * dx + dy * dy > sq_tolerance {
            result.push(*pt);
            prev = pt;
        }
    }
    if result.last() != points.last() {
        result.push(*points.last().unwrap());
    }
    result
}

fn rdp(points: &[[f32; 2]], first: usize, last: usize, sq_tolerance: f32) -> Vec<[f32; 2]> {
    let mut max_sq_dist = 0.0f32;
    let mut max_idx = first;
    let p1 = &points[first];
    let p2 = &points[last];

    for (i, pt) in points[(first + 1)..last].iter().enumerate() {
        let d = get_square_segment_distance(pt, p1, p2);
        if d > max_sq_dist {
            max_sq_dist = d;
            max_idx = first + 1 + i;
        }
    }

    if max_sq_dist > sq_tolerance {
        let left = rdp(points, first, max_idx, sq_tolerance);
        let right = rdp(points, max_idx, last, sq_tolerance);
        let mut result = Vec::with_capacity(left.len() + right.len() - 1);
        result.extend_from_slice(&left[..left.len() - 1]);
        result.extend_from_slice(&right);
        result
    } else {
        vec![*p1, *p2]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simplify_short_line() {
        let pts = vec![[0.0, 0.0], [10.0, 0.0]];
        let result = simplify(&pts, 0.3);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn simplify_reduces_points() {
        let pts = vec![[0.0, 0.0], [1.0, 0.01], [2.0, 0.0], [3.0, 2.0], [4.0, 0.0]];
        let result = simplify(&pts, 0.3);
        assert!(result.len() < pts.len(), "should remove some points");
    }

    #[test]
    fn simplify_preserves_endpoints() {
        let pts = vec![[0.0, 0.0], [50.0, 0.0], [100.0, 0.0]];
        let result = simplify(&pts, 0.3);
        assert_eq!(result.first().unwrap(), &[0.0, 0.0]);
        assert_eq!(result.last().unwrap(), &[100.0, 0.0]);
    }
}
