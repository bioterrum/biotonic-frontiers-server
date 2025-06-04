//! Verifies basic properties of the Elo helper.

use biotonic_server::game::scoring::elo_delta;

#[test]
fn equal_ratings_draw_gives_zero_delta() {
    let (d1, d2) = elo_delta(1500, 1500, 0, 32.0);
    assert_eq!((d1, d2), (0, 0));
}

#[test]
fn winner_gains_and_loser_loses_same_amount() {
    let (d1, d2) = elo_delta(1500, 1500, 1, 32.0); // p1 wins
    assert_eq!(d1, -d2); // conservation of rating
    assert!(d1 > 0 && d2 < 0);
}
