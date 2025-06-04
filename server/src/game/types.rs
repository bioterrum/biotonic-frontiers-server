use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Tracks a player’s resource pools.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ResourcePool {
    pub energy: u32,
    pub biomass: u32,
    pub gene_seeds: u32,
}

/// Four starting archetypes.
#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum UnitType {
    Light,
    Ranged,
    Heavy,
    Seeder,
}

/// One unit on the battlefield.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Unit {
    pub id: Uuid,
    pub unit_type: UnitType,
    pub owner_id: Uuid,
    pub hp: u32, // current hit-points
}

/// Player intent each turn.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum TurnAction {
    PlayUnit {
        unit: Unit,
    },
    Attack {
        attacker_id: Uuid,
        defender_id: Uuid,
    },
    Pass,
}

/// Duel life-cycle.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub enum GameState {
    Lobby,
    InProgress,
    Finished,
}
