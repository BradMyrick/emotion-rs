//! Game logic for the hello-rs demo — a straight port of the hello-ps2
//! C demo's main loop, as pure, FFI-free Rust (unit-testable on host).

use crate::rgbaq;
use libm::sinf;

/// libpad button bits (active-high after the shim's `0xffff ^ btns`).
/// Values must match `ps2sdk/ee/include/libpad.h`.
pub const PAD_CROSS: u32 = 0x4000;
pub const PAD_START: u32 = 0x0008;

const BG: [u32; 6] = [
    rgbaq::rgba(0x10, 0x14, 0x22, 0x00),
    rgbaq::rgba(0x1a, 0x0e, 0x2a, 0x00),
    rgbaq::rgba(0x06, 0x22, 0x18, 0x00),
    rgbaq::rgba(0x28, 0x1a, 0x06, 0x00),
    rgbaq::rgba(0x06, 0x10, 0x28, 0x00),
    rgbaq::rgba(0x00, 0x00, 0x00, 0x00),
];

/// X cycles the sprite color. The bg palettes above are all near-black
/// and sit under the full-screen gradient, so the original demo's
/// bg-only cycle was invisible; the sprite tint makes the input
/// demonstrable (same 6-step cycle, bright colors).
const SPRITE_TINT: [u32; 6] = [
    rgbaq::rgba(0xff, 0xff, 0xff, 0x50),
    rgbaq::rgba(0xff, 0x30, 0x30, 0x50),
    rgbaq::rgba(0x30, 0xff, 0x30, 0x50),
    rgbaq::rgba(0x30, 0xf0, 0xff, 0x50),
    rgbaq::rgba(0xff, 0xf0, 0x30, 0x50),
    rgbaq::rgba(0xff, 0x30, 0xf0, 0x50),
];

/// Everything the game needs to draw one frame. Implemented by the FFI
/// boundary in production and by a call recorder in tests.
pub trait Renderer {
    fn clear(&mut self, rgba: u32);
    fn gradient(&mut self);
    fn gem(&mut self, cx: i32, cy: i32, r: i32, inner_half: i32);
    fn sprite(&mut self, x: i32, y: i32, w: i32, h: i32, rgba: u32);
    fn frame_end(&mut self);
}

pub struct Game {
    width: f32,
    height: f32,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    dx: f32,
    dy: f32,
    t: f32,
    pal: usize,
    old_pad: u32,
}

impl Game {
    pub fn new(width: u32, height: u32) -> Self {
        Game {
            width: width as f32,
            height: height as f32,
            x: 100.0,
            y: 80.0,
            w: 120.0,
            h: 90.0,
            dx: 3.0,
            dy: 2.2,
            t: 0.0,
            pal: 0,
            old_pad: 0,
        }
    }

    /// Advance one frame and issue draw calls to `r`.
    ///
    /// `pad_now` is the current active-high pad bitfield; rising edges
    /// are detected against the previous frame internally.
    ///
    /// Returns `false` when the player pressed START (clean exit); no
    /// frame is drawn in that case, matching the C demo.
    pub fn step(&mut self, pad_now: u32, r: &mut impl Renderer) -> bool {
        let new = pad_now & !self.old_pad;
        self.old_pad = pad_now;

        if new & PAD_CROSS != 0 {
            self.pal = (self.pal + 1) % BG.len();
        }
        if new & PAD_START != 0 {
            return false;
        }

        self.x += self.dx;
        self.y += self.dy;
        if self.x <= 0.0 || self.x + self.w >= self.width {
            self.dx = -self.dx;
        }
        if self.y <= 0.0 || self.y + self.h >= self.height {
            self.dy = -self.dy;
        }
        self.t += 0.05;

        r.clear(BG[self.pal]);
        r.gradient();

        let cx = self.width / 2.0;
        let cy = self.height / 2.0;
        let r_gem = 60.0 + 10.0 * sinf(self.t);
        r.gem(cx as i32, cy as i32, r_gem as i32, 20);

        r.sprite(
            self.x as i32,
            self.y as i32,
            self.w as i32,
            self.h as i32,
            SPRITE_TINT[self.pal],
        );
        r.frame_end();
        true
    }

    /// Current background palette index (0..=5). Test-only accessor.
    #[cfg(test)]
    pub fn palette(&self) -> usize {
        self.pal
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::format;
    use std::string::String;
    use std::string::ToString;
    use std::vec::Vec;

    #[derive(Default)]
    struct Rec(Vec<String>);

    impl Renderer for Rec {
        fn clear(&mut self, rgba: u32) {
            self.0.push(format!("clear {rgba:08x}"));
        }
        fn gradient(&mut self) {
            self.0.push("gradient".to_string());
        }
        fn gem(&mut self, cx: i32, cy: i32, r: i32, inner_half: i32) {
            self.0.push(format!("gem {cx} {cy} {r} {inner_half}"));
        }
        fn sprite(&mut self, x: i32, y: i32, w: i32, h: i32, rgba: u32) {
            self.0.push(format!("sprite {x} {y} {w} {h} {rgba:08x}"));
        }
        fn frame_end(&mut self) {
            self.0.push("flip".to_string());
        }
    }

    fn game640() -> Game {
        Game::new(640, 448)
    }

    #[test]
    fn frame_call_order_matches_demo() {
        let mut g = game640();
        let mut rec = Rec::default();
        assert!(g.step(0, &mut rec));
        assert_eq!(rec.0.len(), 5);
        assert!(rec.0[0].starts_with("clear"));
        assert_eq!(rec.0[1], "gradient");
        assert!(rec.0[2].starts_with("gem 320 224"));
        assert!(rec.0[3].starts_with("sprite 103 82 120 90 50ffffff"));
        assert_eq!(rec.0[4], "flip");
    }

    #[test]
    fn cross_cycles_palette_only_on_rising_edge() {
        let mut g = game640();
        let mut rec = Rec::default();
        assert!(g.step(PAD_CROSS, &mut rec));
        assert_eq!(g.palette(), 1);
        assert!(g.step(PAD_CROSS, &mut rec)); // held, not re-pressed
        assert_eq!(g.palette(), 1);
        assert!(g.step(0, &mut rec)); // release
        assert!(g.step(PAD_CROSS, &mut rec)); // press again
        assert_eq!(g.palette(), 2);
        assert!(g.step(0, &mut rec)); // release
        for _ in 0..4 {
            assert!(g.step(PAD_CROSS, &mut rec)); // press...
            assert!(g.step(0, &mut rec)); // ...and release
        }
        assert_eq!(g.palette(), 0); // wrapped 2 -> 3 -> 4 -> 5 -> 0
    }

    #[test]
    fn cross_press_changes_visible_sprite_color() {
        let mut g = game640();
        let mut rec = Rec::default();

        let sprite_rgba = |rec: &Rec| -> u32 {
            u32::from_str_radix(
                rec.0[rec.0.len() - 2].rsplit(' ').next().unwrap(),
                16,
            )
            .unwrap()
        };

        assert!(g.step(0, &mut rec));
        let first = sprite_rgba(&rec);
        assert_eq!(first, 0x50ff_ffff, "palette 0 is ghost white");

        for expected in [
            0x5030_30ff, // red: rgba(0xff,0x30,0x30,0x50)
            0x5030_ff30, // lime
            0x50ff_f030, // cyan
        ] {
            assert!(g.step(PAD_CROSS, &mut rec)); // press
            assert!(g.step(0, &mut rec)); // release
            assert_eq!(sprite_rgba(&rec), expected);
        }
    }

    #[test]
    fn start_press_exits_without_drawing() {
        let mut g = game640();
        let mut rec = Rec::default();
        assert!(!g.step(PAD_START, &mut rec));
        assert!(rec.0.is_empty());
    }

    #[test]
    fn sprite_bounces_inside_screen() {
        let mut g = game640();
        let mut rec = Rec::default();
        let mut last_x = 100i32;
        let mut going_right = true;
        let mut reversals = 0usize;
        for _ in 0..4000 {
            assert!(g.step(0, &mut rec));
            let x: i32 = rec.0[rec.0.len() - 2][7..]
                .split(' ')
                .next()
                .unwrap()
                .parse()
                .unwrap();
            assert!((-3..=523).contains(&x), "x={x} escaped bounds");
            let d = x - last_x;
            assert!(d.abs() <= 3, "moved {d}px in one frame");
            if d != 0 {
                let now_right = d > 0;
                if now_right != going_right {
                    reversals += 1;
                    going_right = now_right;
                }
            }
            last_x = x;
        }
        assert!(
            reversals > 10,
            "only {reversals} wall bounces in 4000 frames"
        );
    }

    #[test]
    fn gem_radius_pulses_between_50_and_70() {
        let mut g = game640();
        let mut rec = Rec::default();
        for _ in 0..200 {
            assert!(g.step(0, &mut rec));
            let gem = &rec.0[rec.0.len() - 3];
            let r: i32 = gem[4..].split(' ').nth(2).unwrap().parse().unwrap();
            assert!((50..=70).contains(&r), "r={r}");
        }
    }
}
