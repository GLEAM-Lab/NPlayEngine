// 贪吃蛇:选空间最大的方向,再靠近果子。
use npe::Json;

const DIRS: [(i32, i32); 4] = [(0, -1), (1, 0), (0, 1), (-1, 0)];

fn think(obs: &Json) -> String {
    let (w, h) = (obs.int("width"), obs.int("height"));
    let mut wall = vec![vec![false; w as usize]; h as usize];
    for s in obs.get("snakes").items() {
        for c in s.grid("body") {
            wall[c[1] as usize][c[0] as usize] = true;
        }
    }
    let free = |g: &Vec<Vec<bool>>, x: i32, y: i32| x >= 0 && y >= 0 && x < w && y < h && !g[y as usize][x as usize];
    let head = obs.get(&format!("snakes[{}].body[0]", obs.int("me"))).clone();
    let (hx, hy) = (head.int("[0]"), head.int("[1]"));
    let food = obs.grid("food");
    let (mut best, mut dir) = (i32::MIN, 0);
    for (d, (dx, dy)) in DIRS.iter().enumerate() {
        let (x, y) = (hx + dx, hy + dy);
        if !free(&wall, x, y) {
            continue;
        }
        let mut g = wall.clone();
        let mut stack = vec![(x, y)];
        let mut area = 0;
        g[y as usize][x as usize] = true;
        while let Some((a, b)) = stack.pop() {
            area += 1;
            for (dx, dy) in DIRS {
                if free(&g, a + dx, b + dy) {
                    g[(b + dy) as usize][(a + dx) as usize] = true;
                    stack.push((a + dx, b + dy));
                }
            }
        }
        let near = food.iter().map(|f| (f[0] - x).abs() + (f[1] - y).abs()).min().unwrap_or(0);
        let score = area * 10 - near;
        if score > best {
            best = score;
            dir = d as i32;
        }
    }
    npe::action(&[("dir", dir)])
}

npe::bot!(think);
