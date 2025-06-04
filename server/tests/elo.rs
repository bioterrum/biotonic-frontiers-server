//! Unit tests for Elo scoring helpers.

use biotonic_server::game::scoring::elo_delta;

#[test]
fn symmetric_delta_on_draw() {
    let (d1, d2) = elo_delta(1500, 1500, 0, 32.0);
    assert_eq!(d1, 0);
    assert_eq!(d2, 0);
}

#[test]
fn lower_rated_player_gains_more_on_upset() {
    // Player 1 (1400) beats Player 2 (1600)
    let (d1, d2) = elo_delta(1400, 1600, 1, 32.0);
    assert!(d1 > 0);
    assert!(d2 < 0);
    assert_eq!(d1, -d2); // conservation
}
