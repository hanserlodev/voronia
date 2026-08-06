/// Clip a polygon to the map rectangle (Sutherland-Hodgman) with optional "secure".
/// Arithmetic runs in `f64` to match Azgaar's JS exactly (see coast parity Bug 6).
pub fn clip_polygon(points: &[[f32; 2]], width: f32, height: f32, secure: bool) -> Vec<[f32; 2]> {
    let bbox = [0.0f64, 0.0, width as f64, height as f64];
    let pts: Vec<[f64; 2]> = points.iter().map(|p| [p[0] as f64, p[1] as f64]).collect();
    let clipped = sutherland_hodgman(&pts, &bbox);

    let out = if secure {
        secure_points(&clipped, width as f64, height as f64)
    } else {
        clipped
    };
    out.iter().map(|p| [p[0] as f32, p[1] as f32]).collect()
}

fn sutherland_hodgman(points: &[[f64; 2]], bbox: &[f64; 4]) -> Vec<[f64; 2]> {
    let [xmin, ymin, xmax, ymax] = *bbox;
    let mut output = points.to_vec();

    for edge in 0..4 {
        if output.is_empty() {
            return output;
        }
        let input = output;
        output = Vec::with_capacity(input.len());
        let mut prev = input[input.len() - 1];

        for &curr in &input {
            let curr_inside = is_inside(&curr, edge, xmin, ymin, xmax, ymax);
            let prev_inside = is_inside(&prev, edge, xmin, ymin, xmax, ymax);

            if curr_inside {
                if !prev_inside {
                    output.push(intersect(&prev, &curr, edge, xmin, ymin, xmax, ymax));
                }
                output.push(curr);
            } else if prev_inside {
                output.push(intersect(&prev, &curr, edge, xmin, ymin, xmax, ymax));
            }
            prev = curr;
        }
    }

    output
}

fn is_inside(p: &[f64; 2], edge: usize, xmin: f64, ymin: f64, xmax: f64, ymax: f64) -> bool {
    match edge {
        0 => p[0] >= xmin,
        1 => p[0] <= xmax,
        2 => p[1] >= ymin,
        _ => p[1] <= ymax,
    }
}

fn intersect(
    a: &[f64; 2],
    b: &[f64; 2],
    edge: usize,
    xmin: f64,
    ymin: f64,
    xmax: f64,
    ymax: f64,
) -> [f64; 2] {
    match edge {
        0 => {
            let t = (xmin - a[0]) / (b[0] - a[0]);
            [xmin, a[1] + t * (b[1] - a[1])]
        }
        1 => {
            let t = (xmax - a[0]) / (b[0] - a[0]);
            [xmax, a[1] + t * (b[1] - a[1])]
        }
        2 => {
            let t = (ymin - a[1]) / (b[1] - a[1]);
            [a[0] + t * (b[0] - a[0]), ymin]
        }
        _ => {
            let t = (ymax - a[1]) / (b[1] - a[1]);
            [a[0] + t * (b[0] - a[0]), ymax]
        }
    }
}

fn secure_points(points: &[[f64; 2]], width: f64, height: f64) -> Vec<[f64; 2]> {
    let mut secured = Vec::with_capacity(points.len() * 3);
    for &pt in points {
        secured.push(pt);
        if pt[0] <= 0.0 || pt[0] >= width || pt[1] <= 0.0 || pt[1] >= height {
            secured.push(pt);
            secured.push(pt);
        }
    }
    secured
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clip_simple_polygon() {
        let pts = vec![[-10.0, -10.0], [50.0, -10.0], [50.0, 50.0], [-10.0, 50.0]];
        let result = clip_polygon(&pts, 100.0, 100.0, false);
        assert!(result.len() >= 4);
        for p in &result {
            assert!(p[0] >= 0.0 && p[0] <= 100.0, "x out of bounds: {}", p[0]);
            assert!(p[1] >= 0.0 && p[1] <= 100.0, "y out of bounds: {}", p[1]);
        }
    }

    #[test]
    fn clip_inside_polygon() {
        let pts = vec![[10.0, 10.0], [50.0, 10.0], [50.0, 50.0], [10.0, 50.0]];
        let result = clip_polygon(&pts, 100.0, 100.0, false);
        assert_eq!(result.len(), 4);
    }

    #[test]
    fn secure_duplicates_border_points() {
        let pts = vec![[0.0, 0.0], [50.0, 0.0], [50.0, 50.0], [0.0, 50.0]];
        let result = clip_polygon(&pts, 100.0, 100.0, true);
        assert!(result.len() > 4, "secure should duplicate border points");
    }
}
