//! Reference light colors: sRGB at HSV V=1, independent of device dimming.
use serde_json::{json, Value};
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Color {
    pub hue: f64,
    pub saturation: f64,
    pub red: f64,
    pub green: f64,
    pub blue: f64,
    pub x: f64,
    pub y: f64,
}
impl Color {
    pub fn snapshot(self) -> Value {
        json!({"hue":self.hue,"saturation":self.saturation,"red":self.red,"green":self.green,"blue":self.blue,"x":self.x,"y":self.y})
    }
    #[cfg(test)]
    pub fn rgb(self, peak: f64) -> [u8; 3] {
        [self.red, self.green, self.blue].map(|v| (v * peak / 255.).round().clamp(0., 255.) as u8)
    }
}
fn n(v: f64, max: f64, name: &str) -> Result<f64, String> {
    if v.is_finite() && (0. ..=max).contains(&v) {
        Ok(v)
    } else {
        Err(format!("{name} must be between 0 and {max}"))
    }
}
fn lin(v: f64) -> f64 {
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}
fn enc(v: f64) -> f64 {
    if v <= 0.0031308 {
        12.92 * v
    } else {
        1.055 * v.powf(1. / 2.4) - 0.055
    }
}
fn compose(rgb: [f64; 3], hs: Option<(f64, f64)>, xy: Option<(f64, f64)>) -> Result<Color, String> {
    let peak = rgb.into_iter().fold(0., f64::max);
    if peak <= 0. {
        return Err("Black RGB has no color; use onOff to turn the light off".into());
    }
    let [r, g, b] = rgb.map(|v| (v / peak).clamp(0., 1.));
    let d = 1. - r.min(g).min(b);
    let h = if d <= 1e-12 {
        0.
    } else {
        60. * if r == 1. {
            ((g - b) / d).rem_euclid(6.)
        } else if g == 1. {
            (b - r) / d + 2.
        } else {
            (r - g) / d + 4.
        }
    };
    let [rl, gl, bl] = [r, g, b].map(lin);
    let x = 0.4123907992659595 * rl + 0.357584339383878 * gl + 0.1804807884018343 * bl;
    let y = 0.2126390058715104 * rl + 0.715168678767756 * gl + 0.0721923153607337 * bl;
    let z = 0.0193308187155919 * rl + 0.1191947797946259 * gl + 0.9505321522496606 * bl;
    let sum = x + y + z;
    let (h, s) = hs.unwrap_or((h, if d <= 1e-12 { 0. } else { d * 100. }));
    let (x, y) = xy.unwrap_or((x / sum, y / sum));
    Ok(Color {
        hue: h.rem_euclid(360.),
        saturation: s,
        red: r * 255.,
        green: g * 255.,
        blue: b * 255.,
        x,
        y,
    })
}
pub fn from_hs(h: f64, s: f64) -> Result<Color, String> {
    let h = n(h, 360., "hue")?;
    let s = n(s, 100., "saturation")?;
    let hh = h.rem_euclid(360.) / 60.;
    let c = s / 100.;
    let x = c * (1. - (hh % 2. - 1.).abs());
    let m = 1. - c;
    let rgb = if hh < 1. {
        [c, x, 0.]
    } else if hh < 2. {
        [x, c, 0.]
    } else if hh < 3. {
        [0., c, x]
    } else if hh < 4. {
        [0., x, c]
    } else if hh < 5. {
        [x, 0., c]
    } else {
        [c, 0., x]
    };
    compose(rgb.map(|v| (v + m) * 255.), Some((h, s)), None)
}
pub fn from_rgb(r: f64, g: f64, b: f64) -> Result<Color, String> {
    compose(
        [
            n(r, 255., "red")?,
            n(g, 255., "green")?,
            n(b, 255., "blue")?,
        ],
        None,
        None,
    )
}
pub fn from_xy(x: f64, y: f64) -> Result<Color, String> {
    let x = n(x, 1., "x")?;
    let y = n(y, 1., "y")?;
    if y == 0. || x + y > 1. + f64::EPSILON {
        return Err("XY requires y > 0 and x + y <= 1".into());
    }
    let xx = x;
    let yy = y;
    let z = 1. - x - y;
    let rgb = [
        3.2409699419045226 * xx - 1.537383177570094 * yy - 0.4986107602930034 * z,
        -0.9692436362808796 * xx + 1.8759675015077202 * yy + 0.0415550574071756 * z,
        0.0556300796969937 * xx - 0.2039769588889765 * yy + 1.0569715142428786 * z,
    ]
    .map(|v| v.max(0.));
    let peak = rgb.into_iter().fold(0., f64::max);
    if peak <= 0. {
        return Err("XY has no representable reference color".into());
    }
    compose(rgb.map(|v| enc(v / peak) * 255.), None, Some((x, y)))
}
pub fn unknown() -> Value {
    json!({"hue":null,"saturation":null,"red":null,"green":null,"blue":null,"x":null,"y":null})
}
pub fn format(req: &Value) -> Result<&'static str, String> {
    let keys = ["hue", "saturation", "red", "green", "blue", "x", "y"];
    let obj = req.as_object().ok_or("Color request must be an object")?;
    if let Some(k) = obj.keys().find(|k| {
        !keys.contains(&k.as_str()) && !matches!(k.as_str(), "device" | "transitionDuration")
    }) {
        return Err(format!("Unknown color field: {k}"));
    }
    let has = |k: &str| obj.get(k).is_some_and(|v| !v.is_null());
    let hs = has("hue") || has("saturation");
    let rgb = ["red", "green", "blue"].iter().any(|k| has(k));
    let xy = has("x") || has("y");
    if hs as u8 + rgb as u8 + xy as u8 != 1 {
        return Err("Provide exactly one color representation: HS, RGB or XY".into());
    }
    if rgb && !["red", "green", "blue"].iter().all(|k| has(k)) {
        return Err("RGB requires red, green and blue".into());
    }
    if xy && !(has("x") && has("y")) {
        return Err("XY requires x and y".into());
    }
    for (key, max) in [
        ("hue", 360.),
        ("saturation", 100.),
        ("red", 255.),
        ("green", 255.),
        ("blue", 255.),
        ("x", 1.),
        ("y", 1.),
    ] {
        if has(key) {
            n(
                req[key]
                    .as_f64()
                    .ok_or_else(|| format!("{key} must be a number"))?,
                max,
                key,
            )?;
        }
    }
    let kind = if hs {
        "hs"
    } else if rgb {
        "rgb"
    } else {
        "xy"
    };
    if kind == "rgb" {
        from_rgb(
            req["red"].as_f64().unwrap(),
            req["green"].as_f64().unwrap(),
            req["blue"].as_f64().unwrap(),
        )?;
    } else if kind == "xy" {
        from_xy(req["x"].as_f64().unwrap(), req["y"].as_f64().unwrap())?;
    }
    Ok(kind)
}
pub fn target(req: &Value, current: Option<&Value>) -> Result<Color, String> {
    format(req)?;
    if req.get("red").is_some_and(|v| !v.is_null()) {
        return from_rgb(
            req["red"].as_f64().ok_or("red must be a number")?,
            req["green"].as_f64().ok_or("green must be a number")?,
            req["blue"].as_f64().ok_or("blue must be a number")?,
        );
    }
    if req.get("x").is_some_and(|v| !v.is_null()) {
        return from_xy(
            req["x"].as_f64().ok_or("x must be a number")?,
            req["y"].as_f64().ok_or("y must be a number")?,
        );
    }
    let get = |k: &str| {
        req.get(k)
            .filter(|v| !v.is_null())
            .and_then(Value::as_f64)
            .or_else(|| current.and_then(|v| v.get(k)).and_then(Value::as_f64))
            .ok_or_else(|| format!("Current {k} is unavailable to preserve the omitted component"))
    };
    from_hs(get("hue")?, get("saturation")?)
}
pub fn attributes(transition: bool, max_ms: Option<f64>) -> Value {
    let mut settings = vec!["hs", "rgb", "xy"];
    if transition {
        settings.push("transitionDuration")
    }
    let mut result = json!({"supportedStates":["hs","rgb","xy"],"supportedSettings":settings});
    if transition {
        if let Some(ms) = max_ms {
            result["maxTransitionDuration"] = json!({"_type":"Duration","value":ms,"unit":"ms"});
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    fn close(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-7, "{a} != {b}")
    }
    #[test]
    fn rgb_intensity_is_removed_without_quantizing_color() {
        for k in [1., 64., 128., 255.] {
            let c = from_rgb(k, 0., 0.).unwrap();
            assert_eq!(c.hue, 0.);
            assert_eq!(c.saturation, 100.);
            assert_eq!(c.red, 255.);
            close(c.x, 0.64);
            close(c.y, 0.33);
        }
        assert_eq!(
            from_rgb(100., 50., 25.).unwrap(),
            from_rgb(200., 100., 50.).unwrap()
        );
    }
    #[test]
    fn in_gamut_round_trips_cover_fractional_hues_and_saturations() {
        for h in [0., 0.125, 30.5, 120., 180., 240., 359.875, 360.] {
            for s in [1., 12.5, 50., 99.125, 100.] {
                let c = from_hs(h, s).unwrap();
                let rgb = from_rgb(c.red, c.green, c.blue).unwrap();
                close(rgb.hue, c.hue);
                close(rgb.saturation, s);
                let xy = from_xy(c.x, c.y).unwrap();
                close(xy.red, c.red);
                close(xy.green, c.green);
                close(xy.blue, c.blue);
            }
        }
    }
    #[test]
    fn xy_uses_the_native_coordinate_even_outside_reference_rgb_gamut() {
        let c = from_xy(0.75, 0.2).unwrap();
        assert_eq!((c.x, c.y), (0.75, 0.2));
        assert!([c.red, c.green, c.blue]
            .iter()
            .all(|v| v.is_finite() && (0. ..=255.).contains(v)));
    }
    #[test]
    fn tiny_positive_y_never_overflows() {
        let c = from_xy(0.2, 1e-320).unwrap();
        assert!(c.hue.is_finite());
        assert_eq!(c.y, 1e-320);
    }
    #[test]
    fn invalid_composite_requests_fail_before_a_current_read_is_needed() {
        for req in [
            json!({}),
            json!({"red":1}),
            json!({"red":1,"green":0}),
            json!({"x":0.3}),
            json!({"x":0.3,"y":0.3,"hue":5}),
            json!({"hue":50,"red":255,"green":0,"blue":0}),
            json!({"red":0,"green":0,"blue":0}),
            json!({"x":0.8,"y":0.4}),
            json!({"x":0.3,"y":0}),
            json!({"hue":361}),
            json!({"saturation":-1}),
            json!({"hue":"50"}),
            json!({"hue":50,"mode":"rgb"}),
        ] {
            assert!(format(&req).is_err(), "{req}");
            assert!(
                target(&req, Some(&json!({"hue":10,"saturation":20}))).is_err(),
                "{req}"
            );
        }
    }
    #[test]
    fn partial_hs_preserves_the_other_component_and_accepts_json_float_integers() {
        let current = json!({"hue":219.125,"saturation":43.75});
        let h = target(&json!({"hue":50.0}), Some(&current)).unwrap();
        assert_eq!((h.hue, h.saturation), (50., 43.75));
        let s = target(&json!({"saturation":50}), Some(&current)).unwrap();
        assert_eq!((s.hue, s.saturation), (219.125, 50.));
        assert!(target(&json!({"hue":50}), None).is_err());
    }
    #[test]
    fn protocol_rgb_preserves_an_independent_native_intensity() {
        let c = target(&json!({"red":0,"green":128,"blue":0}), None).unwrap();
        assert_eq!(c.rgb(64.), [0, 64, 0]);
        assert_eq!(c.rgb(255.), [0, 255, 0]);
    }
}
