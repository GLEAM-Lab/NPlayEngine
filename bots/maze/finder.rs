// 迷宫拾光:广度优先搜索最近的光点。
use npe::Json;
use std::collections::VecDeque;

const DIRS: [(i32, i32); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];

fn think(obs: &Json) -> String {
    let cells = obs.grid("cells");
    let mut seen = vec![vec![false; cells[0].len()]; cells.len()];
    if obs.int("shield") <= 2 {
        for s in obs.get("sentries").items() {
            let (x, y) = (s.int("x"), s.int("y"));
            seen[y as usize][x as usize] = true;
            for (dx, dy) in DIRS {
                seen[(y + dy) as usize][(x + dx) as usize] = true;
            }
        }
    }
    let mut q = VecDeque::from([(obs.int("x"), obs.int("y"), 4)]);
    while let Some((x, y, first)) = q.pop_front() {
        if first != 4 && cells[y as usize][x as usize] >= 2 {
            return npe::action(&[("dir", first)]);
        }
        for (d, (dx, dy)) in DIRS.iter().enumerate() {
            let (a, b) = (x + dx, y + dy);
            if cells[b as usize][a as usize] == 1 || seen[b as usize][a as usize] {
                continue;
            }
            seen[b as usize][a as usize] = true;
            q.push_back((a, b, if first == 4 { d as i32 } else { first }));
        }
    }
    npe::action(&[("dir", 4)])
}

npe::bot!(think);
