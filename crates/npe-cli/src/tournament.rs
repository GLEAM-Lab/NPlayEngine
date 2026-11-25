//! Round robin. Every group of K bots plays every seat rotation, `rounds` times.

use crate::{load_bots, load_game, toolkit, Args};
use npe_runner::{run_match, BotSpec};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;

#[derive(Default, Clone)]
struct Row {
    matches: u32,
    points: f64,
    wins: u32,
    score: f64,
    faults: u32,
}

fn combinations(n: usize, k: usize) -> Vec<Vec<usize>> {
    if k == 0 {
        return vec![vec![]];
    }
    let mut r = vec![];
    for last in k - 1..n {
        for mut c in combinations(last, k - 1) {
            c.push(last);
            r.push(c);
        }
    }
    r
}

pub fn run(args: &Args) -> Result<(), String> {
    let (game, paths) = args.plain.split_first().ok_or("usage: npe tournament <game> <bot>...")?;
    let g = load_game(game)?;
    let n = paths.len();
    let seats: usize = args.num("seats", n.min(g.spec.max_players))?;
    if seats < g.spec.min_players || seats > g.spec.max_players || seats > n {
        return Err(format!("--seats {seats} does not fit {n} bots and this game"));
    }
    let rounds: u32 = args.num("rounds", 1)?;
    let cores = std::thread::available_parallelism().map_or(2, |c| c.get());
    // Timeouts are wall clock, so each bot needs a core of its own.
    let jobs: usize = args.num("jobs", (cores / seats).max(1))?;
    let tk = toolkit()?;
    let bots = load_bots(&tk, paths)?;

    let mut games = vec![];
    for round in 0..rounds {
        for c in combinations(n, seats) {
            for shift in 0..seats {
                let mut order = c.clone();
                order.rotate_left(shift);
                games.push((round + 1, order));
            }
        }
    }
    let next = AtomicUsize::new(0);
    let table = Mutex::new(vec![Row::default(); n]);
    let failure = Mutex::new(None);
    std::thread::scope(|s| {
        for _ in 0..jobs {
            s.spawn(|| while let Some((seed, order)) = games.get(next.fetch_add(1, Ordering::Relaxed)) {
                let line: Vec<BotSpec> = order
                    .iter()
                    .map(|&b| BotSpec { name: bots[b].name.clone(), lang: bots[b].lang.clone(), command: bots[b].command.clone() })
                    .collect();
                match run_match(&g.spec, &g.rules, *seed, &line) {
                    Ok(r) => {
                        let o = r.outcome.expect("finished match");
                        let mut t = table.lock().unwrap();
                        for (p, &b) in order.iter().enumerate() {
                            let row = &mut t[b];
                            let better = o.ranks.iter().filter(|x| **x < o.ranks[p]).count() as f64;
                            let same = o.ranks.iter().filter(|x| **x == o.ranks[p]).count() as f64 - 1.0;
                            let others = (seats - 1).max(1) as f64;
                            row.matches += 1;
                            row.points += (others - better - same / 2.0) / others;
                            row.wins += u32::from(better == 0.0 && same == 0.0);
                            row.score += o.scores[p];
                            row.faults += r.frames.iter().flat_map(|f| &f.events).filter(|e| e.player == p).count() as u32;
                        }
                    }
                    Err(e) => *failure.lock().unwrap() = Some(e),
                }
            });
        }
    });
    let _ = std::fs::remove_dir_all(&tk.sdks);
    if let Some(e) = failure.into_inner().unwrap() {
        return Err(e);
    }
    let table = table.into_inner().unwrap();
    let mut order: Vec<usize> = (0..n).collect();
    let key = |b: usize| table[b].points / f64::from(table[b].matches.max(1));
    order.sort_by(|a, b| key(*b).total_cmp(&key(*a)));
    println!("{} matches, {seats} seats", games.len());
    println!("{:<4}{:<22}{:<9}{:>7}{:>7}{:>10}{:>8}", "#", "bot", "lang", "points", "wins", "avg score", "faults");
    for (i, b) in order.into_iter().enumerate() {
        let r = &table[b];
        let m = f64::from(r.matches.max(1));
        println!(
            "{:<4}{:<22}{:<9}{:>7.3}{:>7}{:>10.1}{:>8}",
            i + 1, bots[b].name, bots[b].lang, r.points / m, r.wins, r.score / m, r.faults
        );
    }
    Ok(())
}
