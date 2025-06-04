//! Very small Elo helper (K-factor 32 by default)

/// Returns (delta_p1, delta_p2) given current ratings and winner.
/// `winner` = 0 → draw, 1 → p1, 2 → p2.
pub fn elo_delta(r1: i32, r2: i32, winner: u8, k: f32) -> (i32, i32) {
    let e1 = 1.0 / (1.0 + 10f32.powf((r2 - r1) as f32 / 400.0));
    let e2 = 1.0 - e1;
    let (s1, s2) = match winner {
        0 => (0.5, 0.5),
        1 => (1.0, 0.0),
        2 => (0.0, 1.0),
        _ => unreachable!(),
    };
    let d1 = (k * (s1 - e1)).round() as i32;
    let d2 = (k * (s2 - e2)).round() as i32;
    (d1, d2)
}
