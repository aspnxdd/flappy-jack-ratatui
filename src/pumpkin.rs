//! A hollow, carved implicit surface, ray-marched into square half-cell pixels.
use ratatui::{buffer::Buffer, layout::Rect, style::Color};

pub(crate) const BACKGROUND: Color = Color::Rgb(8, 7, 15);
const EYE_LEFT: &[(f32, f32)] = &[(-0.73, 0.14), (-0.20, 0.20), (-0.43, 0.55)];
const EYE_RIGHT: &[(f32, f32)] = &[(0.20, 0.20), (0.73, 0.14), (0.43, 0.55)];
const NOSE: &[(f32, f32)] = &[(-0.12, -0.09), (0.12, -0.09), (0.0, 0.12)];
const MOUTH: &[(f32, f32)] = &[
    (-0.73, -0.16),
    (-0.48, -0.24),
    (-0.40, -0.16),
    (-0.29, -0.30),
    (-0.12, -0.25),
    (0.0, -0.34),
    (0.12, -0.25),
    (0.29, -0.30),
    (0.40, -0.16),
    (0.48, -0.24),
    (0.73, -0.16),
    (0.57, -0.46),
    (0.39, -0.54),
    (0.30, -0.43),
    (0.19, -0.59),
    (-0.19, -0.59),
    (-0.30, -0.43),
    (-0.39, -0.54),
    (-0.57, -0.46),
];

#[derive(Clone, Copy)]
struct V(f32, f32, f32);
impl V {
    fn add(self, b: Self) -> Self {
        Self(self.0 + b.0, self.1 + b.1, self.2 + b.2)
    }
    fn sub(self, b: Self) -> Self {
        Self(self.0 - b.0, self.1 - b.1, self.2 - b.2)
    }
    fn mul(self, k: f32) -> Self {
        Self(self.0 * k, self.1 * k, self.2 * k)
    }
    fn dot(self, b: Self) -> f32 {
        self.0 * b.0 + self.1 * b.1 + self.2 * b.2
    }
    fn length(self) -> f32 {
        self.dot(self).sqrt()
    }
    fn unit(self) -> Self {
        self.mul(1.0 / self.length().max(0.0001))
    }
    fn yaw(self, a: f32) -> Self {
        let (s, c) = a.sin_cos();
        Self(self.0 * c + self.2 * s, self.1, self.2 * c - self.0 * s)
    }
    fn pitch(self, a: f32) -> Self {
        let (s, c) = a.sin_cos();
        Self(self.0, self.1 * c - self.2 * s, self.1 * s + self.2 * c)
    }
}

fn polygon_distance(x: f32, y: f32, polygon: &[(f32, f32)]) -> f32 {
    let mut distance = f32::INFINITY;
    let mut inside = false;
    let mut previous = polygon[polygon.len() - 1];
    for &(bx, by) in polygon {
        let (ax, ay) = previous;
        let (ex, ey) = (bx - ax, by - ay);
        let (px, py) = (x - ax, y - ay);
        let t = ((px * ex + py * ey) / (ex * ex + ey * ey)).clamp(0.0, 1.0);
        distance = distance.min((px - ex * t).powi(2) + (py - ey * t).powi(2));
        if (ay > y) != (by > y) && x < (bx - ax) * (y - ay) / (by - ay) + ax {
            inside = !inside;
        }
        previous = (bx, by);
    }
    distance.sqrt() * if inside { -1.0 } else { 1.0 }
}

fn body_distance(p: V) -> f32 {
    let radius = p.0.hypot(p.2);
    let rib = 1.04 + 0.075 * (10.0 * p.0.atan2(p.2)).cos();
    ((radius / rib).hypot(p.1 / 0.84) - 1.0) * 0.78
}

fn carving_distance(p: V) -> f32 {
    // The bounding box is a conservative early-out, not a replacement for the cuts.
    let bound = (p.0.abs() - 0.74)
        .max(p.1 - 0.56)
        .max(-0.60 - p.1)
        .max(0.18 - p.2);
    if bound > 0.06 {
        return bound;
    }
    let holes = polygon_distance(p.0, p.1, EYE_LEFT)
        .min(polygon_distance(p.0, p.1, EYE_RIGHT))
        .min(polygon_distance(p.0, p.1, NOSE))
        .min(polygon_distance(p.0, p.1, MOUTH));
    holes.max(0.18 - p.2)
}

fn stem_distance(p: V) -> f32 {
    let y = p.1 - 0.93;
    let x = p.0 - 0.25 * y;
    let radius = 0.105 - 0.045 * y;
    let side = x.hypot(p.2) - radius;
    let cap = y.abs() - 0.22;
    side.max(0.0).hypot(cap.max(0.0)) + side.max(cap).min(0.0)
}

fn distance(p: V) -> f32 {
    let shell = (body_distance(p).abs() - 0.045).max(-carving_distance(p));
    shell.min(stem_distance(p))
}

fn normal(p: V) -> V {
    let e = 0.002;
    V(
        distance(p.add(V(e, 0.0, 0.0))) - distance(p.sub(V(e, 0.0, 0.0))),
        distance(p.add(V(0.0, e, 0.0))) - distance(p.sub(V(0.0, e, 0.0))),
        distance(p.add(V(0.0, 0.0, e))) - distance(p.sub(V(0.0, 0.0, e))),
    )
    .unit()
}

fn trace(origin: V, ray: V) -> Option<(V, f32)> {
    // A bounding sphere skips empty space; nearest-hit marching supplies actual depth.
    let b = origin.dot(ray);
    let discriminant = b * b - origin.dot(origin) + 1.42_f32.powi(2);
    if discriminant < 0.0 {
        return None;
    }
    let root = discriminant.sqrt();
    let mut depth = (-b - root).max(0.0);
    let end = -b + root;
    for _ in 0..120 {
        let p = origin.add(ray.mul(depth));
        let d = distance(p);
        if d < 0.0025 {
            return Some((p, depth));
        }
        depth += (d * 0.66).max(0.001);
        if depth > end {
            break;
        }
    }
    None
}

fn rgb(r: f32, g: f32, b: f32) -> Color {
    Color::Rgb(
        r.clamp(0.0, 255.0) as u8,
        g.clamp(0.0, 255.0) as u8,
        b.clamp(0.0, 255.0) as u8,
    )
}

fn shade(p: V, ray: V, yaw: f32, seconds: f32) -> Color {
    let n = normal(p);
    let world_normal = n.yaw(yaw).pitch(0.13);
    let key = world_normal.dot(V(-0.48, 0.67, 0.57).unit()).max(0.0);
    let rim = world_normal.dot(V(0.7, 0.2, -0.65).unit()).max(0.0).powi(3);
    let facing = n.dot(ray.mul(-1.0)).max(0.0);
    let flicker = 0.91 + 0.055 * (seconds * 9.7).sin() + 0.035 * (seconds * 17.3 + 0.6).sin();
    if stem_distance(p) < 0.006 && p.1 > 0.73 {
        let grain = 0.86 + 0.14 * (p.0.atan2(p.2) * 7.0 + p.1 * 8.0).sin();
        return rgb(
            54.0 * (0.35 + key) * grain + rim * 20.0,
            66.0 * (0.3 + key) * grain,
            24.0 * (0.35 + key) + rim * 28.0,
        );
    }
    let interior = body_distance(p) < -0.025;
    let cut_wall = carving_distance(p).abs() < 0.012;
    if interior || cut_wall {
        let light = V(0.0, -0.22, 0.22).sub(p);
        let diffuse = n.dot(light.unit()).max(0.0);
        let warmth = flicker * (0.52 + 0.48 * diffuse) / (0.75 + light.dot(light) * 0.23);
        let edge = if cut_wall { 0.70 } else { 1.0 };
        return rgb(
            255.0 * warmth * edge,
            (133.0 + 82.0 * diffuse) * warmth * edge,
            (22.0 + 40.0 * diffuse) * warmth * edge,
        );
    }
    let groove = 0.82 + 0.18 * (10.0 * p.0.atan2(p.2)).cos();
    let texture = 0.97 + 0.03 * (p.1 * 75.0 + p.0 * 43.0).sin();
    let light = (0.18 + 0.86 * key) * groove * texture;
    let specular = key.powi(12) * facing.powi(3) * 35.0;
    let lip = (-carving_distance(p).abs() * 30.0).exp() * flicker;
    rgb(
        220.0 * light + specular + lip * 48.0 + rim * 24.0,
        75.0 * light + specular * 0.62 + lip * 28.0 + rim * 8.0,
        13.0 * light + specular * 0.15 + lip * 4.0 + rim * 43.0,
    )
}

/// Draw real rotating carved geometry. Each `▀` holds two approximately square RGB pixels.
/// `area` is clipped to the buffer, including buffers whose origin is not (0, 0).
pub fn render(buf: &mut Buffer, area: Rect, seconds: f32) {
    let visible = area.intersection(buf.area);
    if visible.is_empty() || area.is_empty() {
        return;
    }
    let seconds = if seconds.is_finite() {
        seconds.rem_euclid(1000.0)
    } else {
        0.0
    };
    let yaw = 0.16 + seconds * 0.46;
    let scale = (area.width as f32 / 2.9).min(area.height as f32 * 2.0 / 2.65);
    let origin = V(0.0, 0.08, 4.5).pitch(-0.13).yaw(-yaw);
    let bob = (seconds * 1.2).sin() * 0.025;
    let pixel = |x: u16, y: u16| {
        let sx = (x as f32 + 0.5 - area.width as f32 * 0.5) / scale;
        let sy = (area.height as f32 - y as f32 - 0.5) / scale + 0.08 + bob;
        let ray = V(sx, sy, -4.0).unit().pitch(-0.13).yaw(-yaw);
        if let Some((p, _depth)) = trace(origin, ray) {
            return shade(p, ray, yaw, seconds);
        }
        let halo = (-(sx * sx * 0.7 + sy * sy * 0.5)).exp();
        let shadow = (-(sx * sx * 3.5 + (sy + 1.04).powi(2) * 90.0)).exp();
        rgb(
            8.0 + halo * 8.0 - shadow * 7.0,
            7.0 + halo * 3.0 - shadow * 4.0,
            15.0 + halo * 13.0 - shadow * 11.0,
        )
    };
    for y in visible.y..visible.bottom() {
        for x in visible.x..visible.right() {
            let local_x = x - area.x;
            let local_y = (y - area.y) * 2;
            buf[(x, y)]
                .set_symbol("▀")
                .set_fg(pixel(local_x, local_y))
                .set_bg(pixel(local_x, local_y + 1));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn colored_pixels(buf: &Buffer, predicate: impl Fn(u8, u8, u8) -> bool) -> usize {
        buf.content
            .iter()
            .flat_map(|cell| [cell.fg, cell.bg])
            .filter(|color| matches!(color, Color::Rgb(r, g, b) if predicate(*r, *g, *b)))
            .count()
    }

    #[test]
    fn first_frame_has_orange_body_and_bright_carved_apertures() {
        let mut buf = Buffer::empty(Rect::new(0, 0, 76, 19));
        render(&mut buf, Rect::new(0, 0, 76, 19), 0.0);
        assert!(
            colored_pixels(&buf, |r, g, b| r > 65
                && u16::from(r) > u16::from(g) * 2
                && b < 50)
                > 300
        );
        assert!(colored_pixels(&buf, |r, g, b| r > 170 && g > 110 && b < 100) > 35);
    }

    #[test]
    fn carved_shell_and_rotation_have_real_depth() {
        let origin = V(0.0, 0.08, 4.5).pitch(-0.13);
        let ray = V(0.0, -0.05, -1.0).unit().pitch(-0.13);
        let (_, front_depth) = trace(origin, ray).unwrap();
        let (_, back_depth) = trace(
            origin.yaw(-std::f32::consts::PI),
            ray.yaw(-std::f32::consts::PI),
        )
        .unwrap();
        assert!(
            (front_depth - back_depth).abs() > 0.2,
            "a cut reaches the inner back wall"
        );
        let area = Rect::new(0, 0, 64, 18);
        let mut front = Buffer::empty(area);
        let mut turned = Buffer::empty(area);
        render(&mut front, area, 0.0);
        render(&mut turned, area, 2.5);
        assert!(
            front
                .content
                .iter()
                .zip(&turned.content)
                .filter(|(a, b)| a != b)
                .count()
                > 300
        );
    }

    #[test]
    fn clips_to_offset_buffer_and_handles_empty_or_invalid_time() {
        let mut buf = Buffer::empty(Rect::new(10, 20, 8, 5));
        render(&mut buf, Rect::new(8, 18, 12, 9), f32::NAN);
        assert!(buf.content.iter().all(|cell| cell.symbol() == "▀"));
        render(&mut buf, Rect::new(0, 0, 0, 0), 0.0);
        render(&mut buf, Rect::new(0, 0, 2, 2), 0.0);
    }
}
