//! Runs bots as child processes and drives a match over NDJSON pipes.
//!
//! This layer handles timeouts, crashes and cleanup. It is NOT a security
//! sandbox: bots run with the rights of the current user. See docs/SECURITY.md.

mod build;
mod proc;

pub use build::{build, lang_of, BotCommand, Toolkit};

use npe_core::*;
use proc::{Bot, Got};
use std::collections::BTreeMap;
use std::time::Duration;

pub struct BotSpec {
    pub name: String,
    pub lang: String,
    pub command: BotCommand,
}

/// Play one match to the end.
pub fn run_match(spec: &GameSpec, rules: &str, seed: u32, bots: &[BotSpec]) -> Result<Replay, String> {
    let infos = bots.iter().map(|b| PlayerInfo { name: b.name.clone(), lang: b.lang.clone() }).collect();
    let mut m = Match::new(spec.clone(), rules, seed, infos)?;
    let mut procs: Vec<Result<Bot, String>> = bots.iter().map(|b| Bot::spawn(&b.command)).collect();

    // All bots start in parallel and share one deadline for their ready line.
    let init_limit = Duration::from_millis(spec.init_timeout_ms);
    let mut sent = vec![];
    for (p, b) in procs.iter_mut().enumerate() {
        let msg = m.init_message(p);
        sent.push(b.as_mut().ok().map(|b| b.send(&msg)));
    }
    let mut dead: Vec<Option<(FaultKind, String)>> = vec![None; bots.len()];
    for (p, b) in procs.iter_mut().enumerate() {
        match b {
            Err(e) => dead[p] = Some((FaultKind::Crash, e.clone())),
            Ok(b) => match b.recv(sent[p].unwrap(), init_limit, |v| v["ready"] == true) {
                Got::Line(_) => {}
                Got::Timeout => dead[p] = Some((FaultKind::Crash, "no ready line in time".into())),
                Got::Gone(e) => dead[p] = Some((FaultKind::Crash, e)),
            },
        }
    }

    let limit = Duration::from_millis(spec.turn_timeout_ms);
    while !m.finished() {
        let active = m.active()?;
        let turn = m.turn();
        let mut at = BTreeMap::new();
        for &p in &active {
            if let (None, Ok(b)) = (&dead[p], &mut procs[p]) {
                b.turn.store(turn, std::sync::atomic::Ordering::Relaxed);
                at.insert(p, b.send(&m.turn_message(p)?));
            }
        }
        let mut replies = BTreeMap::new();
        for &p in &active {
            let reply = match (&dead[p], &mut procs[p]) {
                (Some((k, d)), _) => Reply::Fault { fault: *k, detail: d.clone() },
                (None, Err(e)) => Reply::Fault { fault: FaultKind::Crash, detail: e.clone() },
                // A late answer to an older turn carries the old number and is skipped.
                (None, Ok(b)) => match b.recv(at[&p], limit, |v| v["turn"] == turn) {
                    Got::Line(mut v) => Reply::Action { action: v["action"].take() },
                    Got::Timeout => Reply::Fault {
                        fault: FaultKind::Timeout,
                        detail: format!("no reply within {} ms", spec.turn_timeout_ms),
                    },
                    Got::Gone(e) => {
                        dead[p] = Some((FaultKind::Crash, e.clone()));
                        Reply::Fault { fault: FaultKind::Crash, detail: e }
                    }
                },
            };
            replies.insert(p, reply);
        }
        m.step(replies)?;
        for (p, b) in procs.iter_mut().enumerate() {
            if let (true, Ok(b)) = (m.is_eliminated(p), b) {
                b.kill();
            }
        }
    }

    let mut replay = m.replay();
    for (p, b) in procs.into_iter().enumerate() {
        if let Ok(b) = b {
            for (turn, text) in b.finish() {
                replay.logs.push(LogLine { turn, player: p, text });
            }
        }
    }
    replay.logs.sort_by_key(|l| l.turn);
    Ok(replay)
}
