//! High-level unit tests for deterministic combat & Elo helpers.

use biotonic_server::game::{
    logic::{resolve_turn, CombatResult},
    scoring,
    types::{ResourcePool, TurnAction, Unit, UnitType},
};
use uuid::Uuid;

fn fresh_pool() -> ResourcePool {
    ResourcePool {
        energy: 5,
        biomass: 5,
        gene_seeds: 2,
    }
}

#[test]
fn play_unit_spawns_and_consumes_resources() {
    let p1 = Uuid::new_v4();

    // Player 1 plays a Light unit; Player 2 passes.
    let light = Unit {
        id: Uuid::new_v4(),
        unit_type: UnitType::Light,
        owner_id: p1,
        hp: 0, // will be filled in by logic
    };
    let actions_p1 = vec![TurnAction::PlayUnit {
        unit: light.clone(),
    }];
    let actions_p2 = vec![TurnAction::Pass];

    // Mutable battle state
    let mut pool1 = fresh_pool();
    let mut pool2 = fresh_pool();
    let mut units1 = Vec::new();
    let mut units2 = Vec::new();

    let CombatResult {
        spawned, applied, ..
    } = resolve_turn(
        actions_p1,
        actions_p2,
        &mut pool1,
        &mut pool2,
        &mut units1,
        &mut units2,
    );

    // Assertions
    assert_eq!(spawned.len(), 1, "one unit should spawn");
    assert_eq!(units1.len(), 1, "unit now on battlefield");
    assert!(applied
        .iter()
        .any(|a| matches!(a, TurnAction::PlayUnit { .. })));
    // A Light unit costs 1 energy: pool should shrink.
    assert_eq!(pool1.energy, 4);
}

#[test]
fn attack_kills_defender() {
    let p1 = Uuid::new_v4();
    let p2 = Uuid::new_v4();

    // Pre-existing units
    let attacker = Unit {
        id: Uuid::new_v4(),
        unit_type: UnitType::Light, // atk 1
        owner_id: p1,
        hp: 1,
    };
    let defender = Unit {
        id: Uuid::new_v4(),
        unit_type: UnitType::Light, // hp 1
        owner_id: p2,
        hp: 1,
    };

    let actions_p1 = vec![TurnAction::Attack {
        attacker_id: attacker.id,
        defender_id: defender.id,
    }];
    let actions_p2 = vec![TurnAction::Pass];

    let mut pool1 = fresh_pool();
    let mut pool2 = fresh_pool();
    let mut units1 = vec![attacker];
    let mut units2 = vec![defender.clone()];

    let res = resolve_turn(
        actions_p1,
        actions_p2,
        &mut pool1,
        &mut pool2,
        &mut units1,
        &mut units2,
    );

    assert!(
        res.destroyed.contains(&defender.id),
        "defender should be destroyed"
    );
    assert!(units2.is_empty(), "defender removed from battlefield");
}

#[test]
fn elo_delta_is_zero_on_draw() {
    let (d1, d2) = scoring::elo_delta(1500, 1500, 0, 32.0);
    assert_eq!(d1, 0);
    assert_eq!(d2, 0);
}

#[test]
fn elo_delta_is_equal_and_opposite_on_win() {
    let (d1, d2) = scoring::elo_delta(1500, 1500, 1, 32.0);
    assert_eq!(d1, -d2, "deltas must sum to zero");
    assert!(d1 > 0, "winner gains points");
}
