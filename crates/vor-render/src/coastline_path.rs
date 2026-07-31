use lyon::geom::point;
use lyon::path::Path;

#[derive(Debug, Clone)]
pub struct CoastlineSpan {
    pub start_idx: usize,
    pub end_idx: usize,
    pub is_smooth: bool,
}

#[derive(Debug, Clone)]
pub enum PathCommand {
    MoveTo(f32, f32),
    LineTo(f32, f32),
    QuadTo(f32, f32, f32, f32),
    CubicTo(f32, f32, f32, f32, f32, f32),
    Close,
}

#[derive(Debug, Clone)]
pub struct CoastlinePath {
    pub commands: Vec<PathCommand>,
}

pub fn build_coastline_path(fractal_points: &[[f32; 2]], spans: &[CoastlineSpan]) -> CoastlinePath {
    let m = spans.len();
    if m == 0 || fractal_points.is_empty() {
        return CoastlinePath {
            commands: Vec::new(),
        };
    }

    let n = fractal_points.len();

    // smooth[i] = (b > a ? b - a : b + n - a) === 1;
    let smooth: Vec<bool> = spans
        .iter()
        .map(|s| {
            let a = s.start_idx;
            let b = s.end_idx;
            (if b > a { b - a } else { b + n - a }) == 1
        })
        .collect();

    let p0 = fractal_points[spans[0].start_idx];
    let p_l = fractal_points[spans[m - 1].start_idx];

    let mut at_mid = smooth[m - 1];
    let (sx, sy) = if at_mid {
        ((p_l[0] + p0[0]) * 0.5, (p_l[1] + p0[1]) * 0.5)
    } else {
        (p0[0], p0[1])
    };

    let mut commands = Vec::new();
    commands.push(PathCommand::MoveTo(sx, sy));

    for i in 0..m {
        let ci = spans[i].start_idx;
        let ni = spans[i].end_idx;
        let (cpx, cpy) = (fractal_points[ci][0], fractal_points[ci][1]);

        if smooth[i] {
            let (npx, npy) = (fractal_points[ni][0], fractal_points[ni][1]);
            let (mx, my) = ((cpx + npx) * 0.5, (cpy + npy) * 0.5);
            if at_mid {
                commands.push(PathCommand::QuadTo(cpx, cpy, mx, my));
            } else {
                commands.push(PathCommand::LineTo(mx, my));
            }
            at_mid = true;
        } else {
            if at_mid {
                commands.push(PathCommand::LineTo(cpx, cpy));
            }
            at_mid = false;

            let end = if ni > ci { ni } else { ni + n };
            for j in ci..end {
                let a = fractal_points[j % n];
                let b = fractal_points[(j + 1) % n];
                let prev = fractal_points[(j + n - 1) % n];
                let nnext = fractal_points[(j + 2) % n];

                let cp1x = a[0] + (b[0] - prev[0]) / 8.0;
                let cp1y = a[1] + (b[1] - prev[1]) / 8.0;
                let cp2x = b[0] - (nnext[0] - a[0]) / 8.0;
                let cp2y = b[1] - (nnext[1] - a[1]) / 8.0;

                commands.push(PathCommand::CubicTo(cp1x, cp1y, cp2x, cp2y, b[0], b[1]));
            }
        }
    }

    commands.push(PathCommand::Close);
    CoastlinePath { commands }
}

pub fn coastline_path_to_lyon(path: &CoastlinePath) -> Path {
    let mut builder = Path::builder();
    for cmd in &path.commands {
        match cmd {
            PathCommand::MoveTo(x, y) => {
                builder.begin(point(*x, *y));
            }
            PathCommand::LineTo(x, y) => {
                builder.line_to(point(*x, *y));
            }
            PathCommand::QuadTo(cx, cy, x, y) => {
                builder.quadratic_bezier_to(point(*cx, *cy), point(*x, *y));
            }
            PathCommand::CubicTo(c1x, c1y, c2x, c2y, x, y) => {
                builder.cubic_bezier_to(point(*c1x, *c1y), point(*c2x, *c2y), point(*x, *y));
            }
            PathCommand::Close => {
                builder.end(true);
            }
        }
    }
    builder.build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smooth_span_produces_quad_bezier() {
        let pts = vec![[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]];
        let spans = vec![
            CoastlineSpan {
                start_idx: 0,
                end_idx: 1,
                is_smooth: true,
            },
            CoastlineSpan {
                start_idx: 1,
                end_idx: 2,
                is_smooth: true,
            },
            CoastlineSpan {
                start_idx: 2,
                end_idx: 3,
                is_smooth: true,
            },
            CoastlineSpan {
                start_idx: 3,
                end_idx: 0,
                is_smooth: true,
            },
        ];
        let result = build_coastline_path(&pts, &spans);
        assert!(
            result.commands.len() > 2,
            "should produce multiple commands"
        );
        let quad_count = result
            .commands
            .iter()
            .filter(|c| matches!(c, PathCommand::QuadTo(..)))
            .count();
        // all 4 smooth spans, all start with at_mid=true, so each emits Q
        assert_eq!(quad_count, 4, "each smooth span should produce one QuadTo");
    }

    #[test]
    fn jagged_span_produces_cubic_bezier() {
        let pts = vec![
            [0.0, 0.0],
            [3.0, 1.0],
            [7.0, -1.0],
            [10.0, 0.0],
            [10.0, 10.0],
            [0.0, 10.0],
        ];
        let spans = vec![
            CoastlineSpan {
                start_idx: 0,
                end_idx: 3,
                is_smooth: false,
            },
            CoastlineSpan {
                start_idx: 3,
                end_idx: 4,
                is_smooth: true,
            },
            CoastlineSpan {
                start_idx: 4,
                end_idx: 5,
                is_smooth: true,
            },
            CoastlineSpan {
                start_idx: 5,
                end_idx: 0,
                is_smooth: true,
            },
        ];
        let result = build_coastline_path(&pts, &spans);
        let cubic_count = result
            .commands
            .iter()
            .filter(|c| matches!(c, PathCommand::CubicTo(..)))
            .count();
        assert!(
            cubic_count > 0,
            "jagged spans should produce CubicTo commands"
        );
    }

    #[test]
    fn start_point_jagged_last_span() {
        // N=7, last span wraps with fractal points → smooth=false → at_mid=false → start at p0
        let pts = vec![
            [0.0, 0.0],   // P0 at idx 0
            [2.0, 1.0],   // f1
            [5.0, -1.0],  // f2
            [10.0, 0.0],  // P1 at idx 3
            [10.0, 10.0], // P2 at idx 4
            [5.0, 9.0],   // P3 at idx 5
            [1.0, 10.0],  // f3 between P3-P0 at idx 6
        ];
        // n=7, m=4
        // Span 3: a=5, b=0 → b+N-a=0+7-5=2>1 → NOT smooth
        let spans = vec![
            CoastlineSpan {
                start_idx: 0,
                end_idx: 3,
                is_smooth: false,
            },
            CoastlineSpan {
                start_idx: 3,
                end_idx: 4,
                is_smooth: true,
            },
            CoastlineSpan {
                start_idx: 4,
                end_idx: 5,
                is_smooth: true,
            },
            CoastlineSpan {
                start_idx: 5,
                end_idx: 0,
                is_smooth: false,
            },
        ];
        let result = build_coastline_path(&pts, &spans);
        assert!(
            matches!(result.commands[0], PathCommand::MoveTo(0.0, 0.0)),
            "jagged last span: start at p0 [0,0], got {:?}",
            result.commands[0]
        );
    }
}
