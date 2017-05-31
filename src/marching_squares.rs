use cairo;
use ndarray::Array2;
use std::ops::Range;

pub fn marching_squares<F>(ctx: &cairo::Context,
                           mut f: F,
                           xr: &Range<f64>,
                           x_len: usize,
                           yr: &Range<f64>,
                           y_len: usize)
                           -> ()
    where F: FnMut(f64, f64) -> f64
{
    assert!(x_len >= 2, "too few x cells: {}", x_len);
    assert!(y_len >= 2, "too few y cells: {}", y_len);
    let x_step = (xr.end - xr.start) / (x_len as f64);
    let y_step = (yr.end - yr.start) / (y_len as f64);
    let arr = Array2::from_shape_fn((x_len + 1, y_len + 1), |(i, j)| {
        let x = xr.start + x_step * (i as f64);
        let y = yr.start + y_step * (j as f64);
        ((x, y), f(x, y))
    });
    // println!("{:6.1?}", arr);

    ctx.set_antialias(cairo::Antialias::Best);
    ctx.set_fill_rule(cairo::FillRule::Winding);
    let (ox, oy, ex, ey) = ctx.clip_extents();

    // canvas coordinates to plot coordinates
    ctx.scale((ex - ox) / (xr.end - xr.start),
              -(ey - oy) / (yr.end - yr.start));
    ctx.translate(-xr.start, -yr.end);

    // clear
    ctx.set_source_rgb(1.0, 1.0, 1.0);
    ctx.paint();

    // axes
    ctx.move_to(xr.start, 0.0);
    ctx.line_to(xr.end, 0.0);
    ctx.move_to(0.0, yr.start);
    ctx.line_to(0.0, yr.end);
    ctx.save();
    ctx.identity_matrix();
    ctx.set_source_rgb(0.0, 0.0, 0.0);
    ctx.set_line_width(1.0);
    ctx.stroke();
    ctx.restore();

    for square in arr.windows((2, 2)) {
        let k = square.iter().fold(0_u8, |acc, &e| (acc << 1) | (e.1.is_sign_positive() as u8));
        // [[8, 4],
        //  [2, 1]]
        let (x0, y0) = square[(0, 0)].0;
        let (x1, y1) = square[(1, 1)].0;
        let v00 = square[(0, 0)].1;
        let v01 = square[(0, 1)].1;
        let v10 = square[(1, 0)].1;
        let v11 = square[(1, 1)].1;
        // println!("{:0>4b}", k);
        assert_eq!(k & 0xF0, 0);
        match k {
            // nothing
            0b1111 | 0b0000 => {}
            // one corner
            0b1000 | 0b0111 => {
                ctx.move_to(x0 + x_step * inv_lerp_0(v00, v10), y0);
                ctx.line_to(x0, y0 + y_step * inv_lerp_0(v00, v01));
            }
            0b0100 | 0b1011 => {
                ctx.move_to(x0 + x_step * inv_lerp_0(v01, v11), y1);
                ctx.line_to(x0, y0 + y_step * inv_lerp_0(v00, v01));
            }
            0b0010 | 0b1101 => {
                ctx.move_to(x0 + x_step * inv_lerp_0(v00, v10), y0);
                ctx.line_to(x1, y0 + y_step * inv_lerp_0(v10, v11));
            }
            0b0001 | 0b1110 => {
                ctx.move_to(x0 + x_step * inv_lerp_0(v01, v11), y1);
                ctx.line_to(x1, y0 + y_step * inv_lerp_0(v10, v11));
            }
            // line
            0b1010 | 0b0101 => {
                ctx.move_to(x0, y0 + y_step * inv_lerp_0(v00, v01));
                ctx.line_to(x1, y0 + y_step * inv_lerp_0(v10, v11));
            }
            0b0011 | 0b1100 => {
                ctx.move_to(x0 + x_step * inv_lerp_0(v00, v10), y0);
                ctx.line_to(x0 + x_step * inv_lerp_0(v01, v11), y1);
            }
            // saddle
            0b0110 | 0b1001 => {
                let center = (v00 + v01 + v10 + v11) / 4.0;
                if (center.is_sign_positive()) == (v00.is_sign_positive()) {
                    ctx.move_to(x0 + x_step * inv_lerp_0(v01, v11), y1);
                    ctx.line_to(x0, y0 + y_step * inv_lerp_0(v00, v01));
                    ctx.move_to(x0 + x_step * inv_lerp_0(v00, v10), y0);
                    ctx.line_to(x1, y0 + y_step * inv_lerp_0(v10, v11));
                } else {
                    ctx.move_to(x0 + x_step * inv_lerp_0(v00, v10), y0);
                    ctx.line_to(x0, y0 + y_step * inv_lerp_0(v00, v01));
                    ctx.move_to(x0 + x_step * inv_lerp_0(v01, v11), y1);
                    ctx.line_to(x1, y0 + y_step * inv_lerp_0(v10, v11));
                }
            }
            _ => unreachable!(),
        }
    }

    ctx.save();
    ctx.identity_matrix();
    ctx.set_source_rgb(1.0, 0.0, 0.0);
    ctx.set_line_width(1.5);
    ctx.stroke();
    ctx.restore();

}

// Returns the value `t` such that `a + t * (b - a) = 0`
fn inv_lerp_0(a: f64, b: f64) -> f64 {
    a / (a - b)
}
