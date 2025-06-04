use crate::game::types::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Resource cost of a unit.
#[derive(Debug, Clone, Copy)]
struct ResourceCost {
    energy: u32,
    biomass: u32,
    gene_seeds: u32,
}

/// Combat stats of a unit.
#[derive(Debug, Clone, Copy)]
struct UnitStats {
    atk: u32,
    hp: u32,
}

impl UnitType {
    fn cost(self) -> ResourceCost {
        match self {
            UnitType::Light => ResourceCost {
                energy: 1,
                biomass: 0,
                gene_seeds: 0,
            },
            UnitType::Ranged => ResourceCost {
                energy: 2,
                biomass: 1,
                gene_seeds: 0,
            },
            UnitType::Heavy => ResourceCost {
                energy: 0,
                biomass: 3,
                gene_seeds: 0,
            },
            UnitType::Seeder => ResourceCost {
                energy: 1,
                biomass: 0,
                gene_seeds: 1,
            },
        }
    }
    fn stats(self) -> UnitStats {
        match self {
            UnitType::Light => UnitStats { atk: 1, hp: 1 },
            UnitType::Ranged => UnitStats { atk: 2, hp: 1 },
            UnitType::Heavy => UnitStats { atk: 3, hp: 3 },
            UnitType::Seeder => UnitStats { atk: 0, hp: 2 },
        }
    }
}

impl ResourcePool {
    fn can_pay(&self, c: ResourceCost) -> bool {
        self.energy >= c.energy && self.biomass >= c.biomass && self.gene_seeds >= c.gene_seeds
    }
    fn pay(&mut self, c: ResourceCost) {
        self.energy -= c.energy;
        self.biomass -= c.biomass;
        self.gene_seeds -= c.gene_seeds;
    }
}

/// Per-turn outcome sent to clients.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CombatResult {
    pub applied: Vec<TurnAction>, // all validated actions
    pub spawned: Vec<Unit>,       // units entering play
    pub destroyed: Vec<Uuid>,     // units killed this turn
}

pub fn resolve_turn(
    actions_p1: Vec<TurnAction>,
    actions_p2: Vec<TurnAction>,
    pool_p1: &mut ResourcePool,
    pool_p2: &mut ResourcePool,
    units_p1: &mut Vec<Unit>,
    units_p2: &mut Vec<Unit>,
) -> CombatResult {
    let mut applied = Vec::new();
    let mut spawned = Vec::new();
    let mut destroyed = Vec::new();

    // 1️⃣  Spawn units (pay cost now).
    let mut play_units =
        |actions: &Vec<TurnAction>, pool: &mut ResourcePool, field: &mut Vec<Unit>| {
            for a in actions {
                if let TurnAction::PlayUnit { unit } = a {
                    let cost = unit.unit_type.cost();
                    if pool.can_pay(cost) {
                        pool.pay(cost);
                        let mut u = unit.clone();
                        u.hp = u.unit_type.stats().hp;
                        field.push(u.clone());
                        spawned.push(u);
                        applied.push(a.clone());
                    }
                }
            }
        };
    play_units(&actions_p1, pool_p1, units_p1);
    play_units(&actions_p2, pool_p2, units_p2);

    // 2️⃣  Collect & sort attacks for deterministic order.
    let mut all_attacks: Vec<TurnAction> = actions_p1
        .iter()
        .chain(actions_p2.iter())
        .filter(|a| matches!(a, TurnAction::Attack { .. }))
        .cloned()
        .collect();
    all_attacks.sort_by_key(|a| match a {
        TurnAction::Attack { attacker_id, .. } => *attacker_id,
        _ => Uuid::nil(),
    });

    // 3️⃣  Apply attacks.
    for action in &all_attacks {
        if let TurnAction::Attack {
            attacker_id,
            defender_id,
        } = *action
        {
            let attacker_in_p1 = units_p1.iter().any(|u| u.id == attacker_id);

            if attacker_in_p1 {
                // attacker is in p1, defender must be in p2
                if let Some(attacker) = units_p1.iter().find(|u| u.id == attacker_id) {
                    let power = attacker.unit_type.stats().atk;
                    if let Some(def_pos) = units_p2.iter().position(|u| u.id == defender_id) {
                        let defender = &mut units_p2[def_pos];
                        if power >= defender.hp {
                            destroyed.push(defender.id);
                            units_p2.remove(def_pos);
                        } else {
                            defender.hp -= power;
                        }
                        applied.push(action.clone());
                    }
                }
            } else {
                // attacker is in p2, defender must be in p1
                if let Some(attacker) = units_p2.iter().find(|u| u.id == attacker_id) {
                    let power = attacker.unit_type.stats().atk;
                    if let Some(def_pos) = units_p1.iter().position(|u| u.id == defender_id) {
                        let defender = &mut units_p1[def_pos];
                        if power >= defender.hp {
                            destroyed.push(defender.id);
                            units_p1.remove(def_pos);
                        } else {
                            defender.hp -= power;
                        }
                        applied.push(action.clone());
                    }
                }
            }
        }
    }

    // 4️⃣  Pass actions are always valid.
    let mut record_passes = |actions: Vec<TurnAction>| {
        for a in actions {
            if matches!(a, TurnAction::Pass) {
                applied.push(a);
            }
        }
    };
    record_passes(actions_p1);
    record_passes(actions_p2);

    CombatResult {
        applied,
        spawned,
        destroyed,
    }
}
