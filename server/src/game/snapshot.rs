//! Serializable per-game snapshot stored in Redis after every turn.

use crate::{
    game::types::{ResourcePool, TurnAction, Unit},
    protocol::ServerMsg,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Key = `game:<game_id>:snap` (JSON)
#[derive(Debug, Serialize, Deserialize)]
pub struct Snapshot {
    pub turn: u32,

    pub p1: Option<Uuid>,
    pub p2: Option<Uuid>,
    pub ready_p1: bool,
    pub ready_p2: bool,

    pub pool_p1: ResourcePool,
    pub pool_p2: ResourcePool,
    pub units_p1: Vec<Unit>,
    pub units_p2: Vec<Unit>,

    pub pending_p1: Option<(u32, Vec<TurnAction>)>,
    pub pending_p2: Option<(u32, Vec<TurnAction>)>,

    pub last_turn_result: Option<ServerMsg>,
}
