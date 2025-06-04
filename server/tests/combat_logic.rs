//! Unit tests for deterministic combat logic.
//!
//! Run with `cargo test -p biotonic-server --tests`.

use biotonic_server::game::{
    logic::resolve_turn,
    types::{ResourcePool, TurnAction, Unit, UnitType},
};
use uuid::Uuid;

fn starter_pool() -> ResourcePool {
    ResourcePool {
        energy: 5,
        biomass: 5,
        gene_seeds: 2,
    }
}

#[test]
fn spawn_unit_pays_cost_and_adds_to_field() {
    let mut pool1 = starter_pool();
    let mut pool2 = starter_pool();
    let mut units1 = Vec::new();
    let mut units2 = Vec::new();

    let light = Unit {
        id: Uuid::new_v4(),
        unit_type: UnitType::Light,
        owner_id: Uuid::nil(),
        hp: 0, // filled in by server on spawn
    };

    let res = resolve_turn(
        vec![TurnAction::PlayUnit {
            unit: light.clone(),
        }],
        vec![TurnAction::Pass],
        &mut pool1,
        &mut pool2,
        &mut units1,
        &mut units2,
    );

    // Exactly one unit spawned and present on the field
    assert_eq!(res.spawned.len(), 1);
    assert!(units1.iter().any(|u| u.id == light.id));

    // Light costs 1 energy → pool now has 4
    assert_eq!(pool1.energy, 4);
}

#[test]
fn heavy_attack_destroys_light_unit() {
    let mut pool1 = starter_pool();
    let mut pool2 = starter_pool();
    let mut units1 = Vec::new();
    let mut units2 = Vec::new();

    // Turn 0 – both players spawn a unit
    let heavy = Unit {
        id: Uuid::new_v4(),
        unit_type: UnitType::Heavy,
        owner_id: Uuid::nil(),
        hp: 0,
    };
    let light = Unit {
        id: Uuid::new_v4(),
        unit_type: UnitType::Light,
        owner_id: Uuid::nil(),
        hp: 0,
    };
    resolve_turn(
        vec![TurnAction::PlayUnit {
            unit: heavy.clone(),
        }],
        vec![TurnAction::PlayUnit {
            unit: light.clone(),
        }],
        &mut pool1,
        &mut pool2,
        &mut units1,
        &mut units2,
    );

    // Turn 1 – heavy one-shots the light
    let res = resolve_turn(
        vec![TurnAction::Attack {
            attacker_id: heavy.id,
            defender_id: light.id,
        }],
        vec![TurnAction::Pass],
        &mut pool1,
        &mut pool2,
        &mut units1,
        &mut units2,
    );

    assert!(res.destroyed.contains(&light.id));
    assert!(!units2.iter().any(|u| u.id == light.id));
}
