use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Parsed `game.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GameSpec {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub description: String,
    pub min_players: usize,
    pub max_players: usize,
    #[serde(default = "d_turn")]
    pub turn_timeout_ms: u64,
    #[serde(default = "d_init")]
    pub init_timeout_ms: u64,
    pub max_turns: u32,
    /// Timeouts and invalid actions beyond this count eliminate the player.
    #[serde(default)]
    pub max_faults: u32,
    /// Passed to the rules script and to bots as-is.
    #[serde(default)]
    pub config: Value,
    /// Passed to the viewer as-is.
    #[serde(default)]
    pub render: Value,
    /// How a human plays in the browser. Passed to the viewer as-is.
    #[serde(default)]
    pub input: Value,
}

fn d_turn() -> u64 {
    500
}
fn d_init() -> u64 {
    10_000
}

impl GameSpec {
    pub fn parse(text: &str) -> Result<Self, String> {
        let s: GameSpec = toml::from_str(text).map_err(|e| format!("game.toml: {e}"))?;
        if s.min_players == 0 || s.min_players > s.max_players || s.max_turns == 0 {
            return Err("game.toml: need 1 <= min_players <= max_players and max_turns > 0".into());
        }
        Ok(s)
    }
}
