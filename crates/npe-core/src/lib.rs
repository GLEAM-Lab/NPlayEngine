//! Deterministic game kernel. No I/O, no clock, no OS randomness.
//! The same (spec, script, seed, replies) always produce the same replay,
//! on native targets and on wasm32.

pub mod rules;
pub mod spec;

pub use rules::Rules;
pub use spec::GameSpec;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

pub type PlayerId = usize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FaultKind {
    Timeout,
    Invalid,
    Crash,
}

/// What a bot produced for one decision.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Reply {
    Action { action: Value },
    Fault { fault: FaultKind, detail: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub player: PlayerId,
    pub kind: FaultKind,
    pub detail: String,
    pub eliminated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Frame {
    pub turn: u32,
    pub view: Value,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub events: Vec<Event>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Outcome {
    /// 1 is best. Ties share a rank.
    pub ranks: Vec<i64>,
    pub scores: Vec<f64>,
    pub reason: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PlayerInfo {
    pub name: String,
    #[serde(default)]
    pub lang: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogLine {
    pub turn: u32,
    pub player: PlayerId,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Replay {
    pub version: u32,
    pub game: String,
    pub title: String,
    pub seed: u32,
    pub players: Vec<PlayerInfo>,
    pub render: Value,
    pub frames: Vec<Frame>,
    pub outcome: Option<Outcome>,
    #[serde(default)]
    pub logs: Vec<LogLine>,
}

pub struct Match {
    pub spec: GameSpec,
    rules: Rules,
    state: rhai::Dynamic,
    seed: u32,
    players: Vec<PlayerInfo>,
    faults: Vec<u32>,
    eliminated: Vec<bool>,
    turn: u32,
    frames: Vec<Frame>,
    outcome: Option<Outcome>,
}

impl Match {
    pub fn new(
        spec: GameSpec,
        script: &str,
        seed: u32,
        players: Vec<PlayerInfo>,
    ) -> Result<Self, String> {
        let n = players.len();
        if n < spec.min_players || n > spec.max_players {
            return Err(format!(
                "{} needs {} to {} players, got {}",
                spec.id, spec.min_players, spec.max_players, n
            ));
        }
        let rules = Rules::compile(script)?;
        let state = rules.init(n, &spec.config, seed)?;
        let mut m = Match {
            rules,
            state,
            seed,
            faults: vec![0; n],
            eliminated: vec![false; n],
            players,
            turn: 0,
            frames: vec![],
            outcome: None,
            spec,
        };
        m.push_frame(vec![])?;
        m.outcome = m.rules.outcome(&m.state, false, n)?;
        Ok(m)
    }

    pub fn turn(&self) -> u32 {
        self.turn
    }
    pub fn finished(&self) -> bool {
        self.outcome.is_some()
    }
    pub fn outcome(&self) -> Option<&Outcome> {
        self.outcome.as_ref()
    }
    pub fn last_frame(&self) -> &Frame {
        self.frames.last().expect("a match always has a frame")
    }
    pub fn is_eliminated(&self, p: PlayerId) -> bool {
        self.eliminated[p]
    }

    /// Players that must act now. Empty once the match is over.
    pub fn active(&self) -> Result<Vec<PlayerId>, String> {
        if self.finished() {
            return Ok(vec![]);
        }
        let mut a = self.rules.active(&self.state)?;
        a.retain(|p| *p < self.players.len() && !self.eliminated[*p]);
        a.sort_unstable();
        a.dedup();
        Ok(a)
    }

    pub fn observe(&self, p: PlayerId) -> Result<Value, String> {
        self.rules.observe(&self.state, p)
    }

    /// First message a bot receives.
    pub fn init_message(&self, p: PlayerId) -> Value {
        serde_json::json!({
            "type": "init", "game": self.spec.id, "player": p,
            "players": self.players.len(), "turn_timeout_ms": self.spec.turn_timeout_ms,
            "config": self.spec.config,
        })
    }

    pub fn turn_message(&self, p: PlayerId) -> Result<Value, String> {
        Ok(serde_json::json!({"type": "turn", "turn": self.turn, "obs": self.observe(p)?}))
    }

    /// Advance one step. Every active player must have a reply; a missing one is a timeout.
    pub fn step(&mut self, mut replies: BTreeMap<PlayerId, Reply>) -> Result<(), String> {
        let active = self.active()?;
        if self.finished() {
            return Err("match is already over".into());
        }
        let n = self.players.len();
        let mut actions: Vec<Option<Value>> = vec![None; n];
        let mut events = vec![];
        let mut to_eliminate = vec![];
        for p in active {
            let (kind, detail) = match replies.remove(&p) {
                Some(Reply::Action { action }) => {
                    match self.rules.validate(&self.state, p, &action) {
                        Ok(()) => {
                            actions[p] = Some(action);
                            continue;
                        }
                        Err(e) => (FaultKind::Invalid, e),
                    }
                }
                Some(Reply::Fault { fault, detail }) => (fault, detail),
                None => (FaultKind::Timeout, "no reply".into()),
            };
            self.faults[p] += 1;
            let out = kind == FaultKind::Crash || self.faults[p] > self.spec.max_faults;
            events.push(Event { player: p, kind, detail, eliminated: out });
            if out {
                to_eliminate.push(p);
            } else {
                actions[p] = Some(self.rules.default_action(&self.state, p)?);
            }
        }
        for p in to_eliminate {
            self.eliminated[p] = true;
            self.state = self.rules.eliminate(self.state.clone(), p)?;
        }
        self.outcome = self.rules.outcome(&self.state, false, n)?;
        if self.outcome.is_none() && actions.iter().any(Option::is_some) {
            self.state = self.rules.apply(self.state.clone(), &actions)?;
        }
        self.turn += 1;
        if self.outcome.is_none() {
            self.outcome = self.rules.outcome(&self.state, false, n)?;
        }
        let stuck = self.outcome.is_none() && self.active()?.is_empty();
        if self.outcome.is_none() && (stuck || self.turn >= self.spec.max_turns) {
            self.outcome = self.rules.outcome(&self.state, true, n)?;
            if self.outcome.is_none() {
                return Err("outcome(state, true) must return a result".into());
            }
        }
        self.push_frame(events)
    }

    fn push_frame(&mut self, events: Vec<Event>) -> Result<(), String> {
        let view = self.rules.view(&self.state)?;
        self.frames.push(Frame { turn: self.turn, view, events });
        Ok(())
    }

    pub fn replay(&self) -> Replay {
        Replay {
            version: 1,
            game: self.spec.id.clone(),
            title: self.spec.title.clone(),
            seed: self.seed,
            players: self.players.clone(),
            render: self.spec.render.clone(),
            frames: self.frames.clone(),
            outcome: self.outcome.clone(),
            logs: vec![],
        }
    }
}
