use biotonic_server::game::scoring::elo_delta;

#[test]
fn elo_winner_gets_positive_delta() {
    let (d1, d2) = elo_delta(1500, 1500, 1, 32.0);
    assert!(d1 > 0 && d2 < 0 && d1.abs() == d2.abs());
}

#[test]
fn elo_draw_is_zero_sum() {
    let (d1, d2) = elo_delta(1600, 1400, 0, 32.0);
    assert_eq!(d1 + d2, 0);
}
