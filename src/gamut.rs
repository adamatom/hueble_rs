use palette;

// Philips Hue Gamuts, https://homeautotechs.com/philips-hue-light-models-full-list/

pub const PHILIPS_GAMUT_A: PhilipsGamut = PhilipsGamut {
    red: (0.704, 0.296),
    green: (0.2151, 0.7106),
    blue: (0.138, 0.08),
};

pub const PHILIPS_GAMUT_B: PhilipsGamut = PhilipsGamut {
    red: (0.675, 0.322),
    green: (0.4091, 0.518),
    blue: (0.167, 0.04),
};

pub const PHILIPS_GAMUT_C: PhilipsGamut = PhilipsGamut {
    red: (0.692, 0.308),
    green: (0.17, 0.7),
    blue: (0.153, 0.048),
};

pub trait ClampToGamut<T> {
    fn clamp_to(&self, gamut: &PhilipsGamut) -> T;
}

impl ClampToGamut<palette::Yxy> for palette::Yxy {
    fn clamp_to(&self, gamut: &PhilipsGamut) -> palette::Yxy {
        if gamut.is_inside((self.x, self.y)) {
            *self
        } else {
            let closest_point = gamut.closest_point((self.x, self.y));
            palette::Yxy::new(closest_point.0, closest_point.1, self.luma)
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PhilipsGamut {
    pub red: (f32, f32),
    pub green: (f32, f32),
    pub blue: (f32, f32),
}

impl PhilipsGamut {
    pub fn closest_point(&self, p: (f32, f32)) -> (f32, f32) {
        let green_to_blue = closest_point_on_segment(p, self.green, self.blue);
        let green_to_red = closest_point_on_segment(p, self.green, self.red);
        let blue_to_red = closest_point_on_segment(p, self.blue, self.red);

        let dist_to_gb = dist(p, green_to_blue);
        let dist_to_gr = dist(p, green_to_red);
        let dist_to_br = dist(p, blue_to_red);

        if dist_to_gb < dist_to_gr && dist_to_gb < dist_to_br {
            green_to_blue
        } else if dist_to_gr < dist_to_br {
            green_to_red
        } else {
            blue_to_red
        }
    }

    pub fn is_inside(&self, p: (f32, f32)) -> bool {
        let v0 = sub(self.blue, self.red);
        let v1 = sub(self.green, self.red);
        let v2 = sub((p.0, p.1), self.red);

        let dot00 = v0.0 * v0.0 + v0.1 * v0.1;
        let dot01 = v0.0 * v1.0 + v0.1 * v1.1;
        let dot02 = v0.0 * v2.0 + v0.1 * v2.1;
        let dot11 = v1.0 * v1.0 + v1.1 * v1.1;
        let dot12 = v1.0 * v2.0 + v1.1 * v2.1;

        let inv_denom = 1.0 / (dot00 * dot11 - dot01 * dot01);

        let u = (dot11 * dot02 - dot01 * dot12) * inv_denom;
        let v = (dot00 * dot12 - dot01 * dot02) * inv_denom;
        u >= 0.0 && v >= 0.0 && (u + v) < 1.0
    }
}

fn closest_point_on_segment(p: (f32, f32), a: (f32, f32), b: (f32, f32)) -> (f32, f32) {
    if a == b {
        // Avoid division by zero if a and b are the same point
        return a;
    }

    let ap = sub(p, a);
    let ab = sub(b, a);
    let ab_sqr = dot(ab, ab);
    let ap_dot_ab = dot(ap, ab);
    let t = ap_dot_ab / ab_sqr;

    // Clamp t to the range [0.0, 1.0] to stay on the segment
    add(a, scale(ab, t.clamp(0.0, 1.0)))
}

fn sub(a: (f32, f32), b: (f32, f32)) -> (f32, f32) {
    (a.0 - b.0, a.1 - b.1)
}

fn add(a: (f32, f32), b: (f32, f32)) -> (f32, f32) {
    (a.0 + b.0, a.1 + b.1)
}

fn dot(a: (f32, f32), b: (f32, f32)) -> f32 {
    a.0 * b.0 + a.1 * b.1
}

fn scale(a: (f32, f32), s: f32) -> (f32, f32) {
    (a.0 * s, a.1 * s)
}

fn dist(a: (f32, f32), b: (f32, f32)) -> f32 {
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    (dx * dx + dy * dy).sqrt()
}
