use npe_core::*;
use serde_json::{json, Value};
use std::collections::BTreeMap;

const GAMES: &[(&str, usize, usize)] = &[("reversi", 2, 2), ("snake", 1, 6), ("maze", 1, 4)];

fn load(id: &str, n: usize, seed: u32) -> Match {
    let dir = format!("{}/../../games/{id}", env!("CARGO_MANIFEST_DIR"));
    let spec = GameSpec::parse(&std::fs::read_to_string(format!("{dir}/game.toml")).unwrap()).unwrap();
    let rules = std::fs::read_to_string(format!("{dir}/rules.rhai")).unwrap();
    let players = (0..n).map(|i| PlayerInfo { name: format!("P{i}"), lang: "test".into() }).collect();
    Match::new(spec, &rules, seed, players).unwrap()
}

fn act(v: Value) -> Reply {
    Reply::Action { action: v }
}

fn crash() -> Reply {
    Reply::Fault { fault: FaultKind::Crash, detail: "segfault".into() }
}

/// Deterministic policy. `scripts/check-wasm.mjs` repeats it in JavaScript.
fn scripted(id: &str, obs: &Value, turn: u32, p: usize) -> Value {
    match id {
        "reversi" => obs["moves"][turn as usize % obs["moves"].as_array().unwrap().len()].clone(),
        _ => json!({"dir": (turn as usize / 4 + p) % 4}),
    }
}

fn play(id: &str, n: usize, seed: u32) -> Match {
    let mut m = load(id, n, seed);
    while !m.finished() {
        let t = m.turn();
        let r = m.active().unwrap().into_iter().map(|p| (p, act(scripted(id, &m.observe(p).unwrap(), t, p)))).collect();
        m.step(r).unwrap();
    }
    m
}

#[test]
fn reversi_opening_flips_one_piece() {
    let mut m = load("reversi", 2, 1);
    assert_eq!(m.observe(0).unwrap()["moves"].as_array().unwrap().len(), 4);
    m.step(BTreeMap::from([(0, act(json!({"x": 2, "y": 3})))])).unwrap();
    let players = &m.last_frame().view["players"];
    assert_eq!((players[0]["score"].as_i64(), players[1]["score"].as_i64()), (Some(4), Some(1)));
    assert_eq!(m.active().unwrap(), [1]);
}

#[test]
fn reversi_rejects_a_move_that_flips_nothing() {
    let mut m = load("reversi", 2, 1);
    m.step(BTreeMap::from([(0, act(json!({"x": 0, "y": 0})))])).unwrap();
    assert_eq!(m.outcome().unwrap().ranks, [2, 1]);
}

#[test]
fn reversi_shortest_wipeout_ends_early() {
    // A known nine-move game that leaves the second player without pieces.
    let mut m = load("reversi", 2, 1);
    for mv in ["e6", "f4", "e3", "f6", "g5", "d6", "e7", "f5", "c5"] {
        // That game starts from the mirrored opening, so mirror the column.
        let b = mv.as_bytes();
        let (x, y) = (i64::from(b[0] - b'a'), i64::from(b[1] - b'1'));
        let p = m.active().unwrap()[0];
        m.step(BTreeMap::from([(p, act(json!({"x": x, "y": y})))])).unwrap();
        assert!(m.last_frame().events.is_empty(), "{mv}: {:?}", m.last_frame().events);
    }
    let o = m.outcome().expect("wipeout");
    assert_eq!((o.ranks.clone(), o.scores[1]), (vec![1, 2], 0.0));
}

#[test]
fn malformed_actions_never_break_the_match() {
    let bad = [
        json!(null), json!("x"), json!([]), json!({}), json!({"x": 1.5, "y": 0}), json!({"x": -1, "y": 0}),
        json!({"x": 1e300, "y": 0}), json!({"dir": 9}), json!({"dir": -1}), json!({"dir": "up"}), json!({"dir": 1.0}),
        json!({"x": i64::MAX, "y": 1}), json!({"dir": u64::MAX}), json!({"dir": i64::MIN}),
        json!({"dir": {"dir": {"dir": 1}}}),
    ];
    for &(id, _, n) in GAMES {
        for b in &bad {
            let mut m = load(id, n, 5);
            let r = m.active().unwrap().into_iter().map(|p| (p, act(b.clone()))).collect();
            m.step(r).unwrap();
            assert_eq!(m.last_frame().events[0].kind, FaultKind::Invalid, "{id} {b}");
        }
    }
}

#[test]
fn a_crash_removes_one_player_and_the_rest_go_on() {
    let mut m = load("snake", 4, 3);
    let mut r: BTreeMap<_, _> = (0..4).map(|p| (p, act(json!({"dir": 1})))).collect();
    r.insert(2, crash());
    m.step(r).unwrap();
    assert!(!m.finished());
    assert!(!m.active().unwrap().contains(&2));
    let e = &m.last_frame().events[0];
    assert!(e.player == 2 && e.eliminated && e.kind == FaultKind::Crash);
}

#[test]
fn everyone_crashing_still_ends_the_match() {
    for &(id, lo, hi) in GAMES {
        for n in lo..=hi {
            let mut m = load(id, n, 3);
            while !m.finished() {
                let r = m.active().unwrap().into_iter().map(|p| (p, crash())).collect();
                m.step(r).unwrap();
            }
        }
    }
}

#[test]
fn timeouts_use_the_default_action_until_the_limit() {
    let mut m = load("maze", 2, 3);
    let limit = m.spec.max_faults;
    for i in 0..=limit {
        m.step(BTreeMap::from([(0, act(json!({"dir": 4})))])).unwrap();
        assert_eq!(m.is_eliminated(1), i == limit);
    }
}

#[test]
fn every_seat_count_plays_to_the_end_deterministically() {
    for &(id, lo, hi) in GAMES {
        for n in lo..=hi {
            for seed in [0, 42, u32::MAX] {
                let a = serde_json::to_string(&play(id, n, seed).replay()).unwrap();
                let b = serde_json::to_string(&play(id, n, seed).replay()).unwrap();
                assert_eq!(a, b, "{id} {n}");
            }
        }
    }
}

#[test]
fn every_maze_is_connected() {
    for seed in 0..200 {
        let m = load("maze", 4, seed);
        let cells: Vec<i64> = m.last_frame().view["cells"].as_array().unwrap().iter().map(|v| v.as_i64().unwrap()).collect();
        let open = cells.iter().filter(|v| **v != 1).count();
        let mut seen = vec![false; cells.len()];
        let mut stack = vec![22usize];
        seen[22] = true;
        let mut k = 0;
        while let Some(i) = stack.pop() {
            k += 1;
            for j in [i - 21, i + 1, i + 21, i - 1] {
                if cells[j] != 1 && !seen[j] {
                    seen[j] = true;
                    stack.push(j);
                }
            }
        }
        assert_eq!(k, open, "seed {seed}");
        // Sentries never start on a wall.
        for e in m.last_frame().view["entities"].as_array().unwrap() {
            let i = e["y"].as_u64().unwrap() * 21 + e["x"].as_u64().unwrap();
            assert_ne!(cells[i as usize], 1, "seed {seed}");
        }
    }
}

#[test]
fn fog_hides_distant_players() {
    let m = load("maze", 2, 1);
    assert_eq!(m.observe(0).unwrap()["others"], json!([]));
    assert_eq!(m.observe(0).unwrap()["sentries"], json!([]));
}

#[test]
fn runaway_rules_are_stopped() {
    let src = std::fs::read_to_string(format!("{}/../../games/snake/rules.rhai", env!("CARGO_MANIFEST_DIR"))).unwrap();
    let src = src.replace("fn active(s) {", "fn active(s) { loop { } ");
    let spec = load("snake", 1, 1).spec;
    assert!(Match::new(spec, &src, 1, vec![PlayerInfo::default()]).unwrap().active().is_err());
}

/// Golden files for the native/wasm comparison.
#[test]
fn write_golden_replays() {
    let out = format!("{}/../../target/golden", env!("CARGO_MANIFEST_DIR"));
    std::fs::create_dir_all(&out).unwrap();
    for &(id, _, n) in GAMES {
        let r = serde_json::to_string(&play(id, n, 42).replay()).unwrap();
        std::fs::write(format!("{out}/{id}.json"), r).unwrap();
    }
}
