//! Runtime configuration for the Biotonic Frontiers server.

use once_cell::sync::Lazy;
use std::env;

#[derive(Debug)]
pub struct Settings {
    /// Maximum duel length before auto-finish.
    pub max_turns: u32,
    /// Redis presence-key TTL (seconds).
    pub presence_ttl: u64,
    /// Seconds a player may stay disconnected before forfeit.
    pub disconnect_grace: u64,
}

impl Settings {
    fn from_env() -> Self {
        let max_turns = env::var("MAX_TURNS")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(5);

        let presence_ttl = env::var("PRESENCE_TTL")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(600);

        let disconnect_grace = env::var("DISCONNECT_GRACE")
            .ok()
            .and_then(|v| v.parse::<u64>().ok())
            .unwrap_or(120); // 2 min default

        Settings {
            max_turns,
            presence_ttl,
            disconnect_grace,
        }
    }
}

static SETTINGS: Lazy<Settings> = Lazy::new(Settings::from_env);

pub fn settings() -> &'static Settings {
    &SETTINGS
}
