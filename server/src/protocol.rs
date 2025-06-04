//! Wire-protocol shared by client, WS handler and game session.

use crate::game::{logic::CombatResult, types::TurnAction};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------- client → server ----------
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMsg {
    Ready {
        game_id: Uuid,
        player_id: Uuid,
    },
    Turn {
        game_id: Uuid,
        player_id: Uuid,
        turn: u32,
        actions: Vec<TurnAction>,
    },
    /// Sent by a client that lost its socket and re-opened a new one.
    Resume {
        game_id: Uuid,
        player_id: Uuid,
    },
    /// Emitted internally by the WS layer when a socket closes.
    Disconnected {
        game_id: Uuid,
        player_id: Uuid,
    },
}

// ---------- server → client ----------
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "type")]
pub enum ServerMsg {
    GameStart {
        game_id: Uuid,
        turn: u32,
    },
    TurnResult {
        game_id: Uuid,
        turn: u32,
        result: CombatResult,
    },
    GameOver {
        game_id: Uuid,
        winner: Option<Uuid>,
    },

    /// New: real-time faction chat
    FactionChat {
        faction_id: Uuid,
        sender_id: Uuid,
        content: String,
        ts: DateTime<Utc>,
    },
}
