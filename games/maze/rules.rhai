// Collect lights in a generated maze while sentries patrol.
// Cells: 0 empty, 1 wall, 2 light, 3 shield. dir: 0 up, 1 right, 2 down, 3 left, 4 stay.

fn init(ctx) {
    let c = ctx.config;
    let s = #{ w: c.width, h: c.height, n: ctx.players, c: c, rng: rng_next(ctx.seed), turn: 0, cells: [], runners: [], sentries: [], left: 0 };
    s = build(s);
    let w = s.w;
    let h = s.h;
    let starts = [[1, 1], [w - 2, h - 2], [w - 2, 1], [1, h - 2]];
    for p in 0..s.n {
        let x = starts[p][0];
        let y = starts[p][1];
        s.cells[y * w + x] = 0;
        s.runners.push(#{ x: x, y: y, sx: x, sy: y, score: 0, lives: c.lives, shield: 0, alive: true, why: "" });
    }
    let mx = w / 2;
    let my = h / 2;
    if my % 2 == 0 { my += 1; }
    for k in 0..c.sentries {
        s.sentries.push(#{ x: mx, y: my, hx: mx, hy: my, wait: 4 + k * 3 });
    }
    for v in s.cells { if v == 2 { s.left += 1; } }
    s
}

// Depth-first maze on odd cells, opened up with extra gaps, then mirrored so
// the left and right halves are fair.
fn build(s) {
    let w = s.w;
    let h = s.h;
    let mid = w / 2;
    let cells = [];
    cells.pad(w * h, 1);
    let stack = [[1, 1]];
    cells[w + 1] = 2;
    while stack.len() > 0 {
        let cur = stack[stack.len() - 1];
        let opts = [];
        for d in [[0, -2], [2, 0], [0, 2], [-2, 0]] {
            let x = cur[0] + d[0];
            let y = cur[1] + d[1];
            if x > 0 && y > 0 && x < mid && y < h - 1 && cells[y * w + x] == 1 { opts.push([x, y]); }
        }
        if opts.len() == 0 { stack.pop(); continue; }
        s.rng = rng_next(s.rng);
        let t = opts[s.rng % opts.len()];
        cells[t[1] * w + t[0]] = 2;
        cells[(t[1] + cur[1]) / 2 * w + (t[0] + cur[0]) / 2] = 2;
        stack.push(t);
    }
    for y in 1..(h - 1) {
        for x in 1..mid {
            if cells[y * w + x] != 1 || (x + y) % 2 == 0 { continue; }
            s.rng = rng_next(s.rng);
            if s.rng % 100 < 28 { cells[y * w + x] = 2; }
        }
    }
    for y in 0..h {
        for x in 0..mid { cells[y * w + w - 1 - x] = cells[y * w + x]; }
    }
    // Middle column: open on every odd row, plus a hall for the sentries.
    for y in 1..(h - 1) {
        if y % 2 == 1 { cells[y * w + mid] = 2; }
    }
    let placed = 0;
    let tries = 0;
    while placed < s.c.shields && tries < 200 {
        tries += 1;
        s.rng = rng_next(s.rng);
        let i = s.rng % (w * h);
        if cells[i] == 2 && i % w > 2 && i % w < mid {
            cells[i] = 3;
            cells[i / w * w + w - 1 - i % w] = 3;
            placed += 2;
        }
    }
    s.cells = cells;
    s
}

fn active(s) {
    let r = [];
    for p in 0..s.n { if s.runners[p].alive { r.push(p); } }
    r
}

fn near(s, a, b) { (a.x - b.x).abs() + (a.y - b.y).abs() <= s.c.sight }

fn observe(s, p) {
    let me = s.runners[p];
    let rows = [];
    for y in 0..s.h { rows.push(s.cells.extract(y * s.w, s.w)); }
    let others = [];
    for q in 0..s.n {
        let r = s.runners[q];
        if q != p && r.alive && near(s, me, r) { others.push(#{ id: q, x: r.x, y: r.y, shield: r.shield }); }
    }
    let sentries = [];
    for t in s.sentries { if near(s, me, t) { sentries.push(#{ x: t.x, y: t.y }); } }
    let scores = [];
    for r in s.runners { scores.push(r.score); }
    #{
        turn: s.turn, width: s.w, height: s.h, me: p, x: me.x, y: me.y, shield: me.shield, lives: me.lives,
        scores: scores, cells: rows, others: others, sentries: sentries
    }
}

fn validate(s, p, a) {
    if type_of(a) != "map" || type_of(a.dir) != "i64" { return "action must be {dir} with an integer"; }
    if a.dir < 0 || a.dir > 4 { return `dir must be 0 to 4, got ${a.dir}`; }
    ""
}

fn default_action(s, p) { #{ dir: 4 } }

fn eliminate(s, p) {
    s.runners[p].alive = false;
    s.runners[p].why = "程序出局";
    s
}

fn shift(x, y, d) {
    if d == 0 { [x, y - 1] } else if d == 1 { [x + 1, y] } else if d == 2 { [x, y + 1] } else if d == 3 { [x - 1, y] } else { [x, y] }
}

// Steps from the nearest living runner to every open cell.
fn field(s) {
    let dist = [];
    dist.pad(s.w * s.h, 999);
    let queue = [];
    for r in s.runners {
        if r.alive && dist[r.y * s.w + r.x] != 0 { dist[r.y * s.w + r.x] = 0; queue.push(r.y * s.w + r.x); }
    }
    let k = 0;
    while k < queue.len() {
        let i = queue[k];
        k += 1;
        for d in [-s.w, 1, s.w, -1] {
            let j = i + d;
            if s.cells[j] != 1 && dist[j] == 999 { dist[j] = dist[i] + 1; queue.push(j); }
        }
    }
    dist
}

fn apply(s, actions) {
    s.turn += 1;
    let from = [];
    for p in 0..s.n {
        let r = s.runners[p];
        from.push([r.x, r.y]);
        if !r.alive || actions[p] == () { continue; }
        let t = shift(r.x, r.y, actions[p].dir);
        if s.cells[t[1] * s.w + t[0]] != 1 { s.runners[p].x = t[0]; s.runners[p].y = t[1]; }
        if r.shield > 0 { s.runners[p].shield -= 1; }
    }
    // Runners that reach the same light split nothing: each of them scores it.
    let taken = [];
    for p in 0..s.n {
        let r = s.runners[p];
        if !r.alive { continue; }
        let i = r.y * s.w + r.x;
        if s.cells[i] == 2 { s.runners[p].score += 1; taken.push(i); }
        if s.cells[i] == 3 { s.runners[p].score += 5; s.runners[p].shield = s.c.shield_turns; taken.push(i); }
    }
    for i in taken {
        if s.cells[i] == 2 { s.left -= 1; }
        s.cells[i] = 0;
    }
    let dist = field(s);
    for k in 0..s.sentries.len() {
        let t = s.sentries[k];
        let old = [t.x, t.y];
        if t.wait > 0 {
            s.sentries[k].wait -= 1;
        } else if (s.turn + k) % 4 != 0 {
            s = move_sentry(s, k, dist);
        }
        s = collide(s, k, old, from);
    }
    s
}

fn move_sentry(s, k, dist) {
    let t = s.sentries[k];
    let opts = [];
    for d in 0..4 {
        let q = shift(t.x, t.y, d);
        if s.cells[q[1] * s.w + q[0]] != 1 { opts.push(q); }
    }
    if opts.len() == 0 { return s; }
    s.rng = rng_next(s.rng);
    let pick = opts[s.rng % opts.len()];
    // Sentry 0 always hunts. The others wander more the higher their index.
    s.rng = rng_next(s.rng);
    if s.rng % 100 >= 25 * k {
        let scared = any_shield(s);
        for q in opts {
            let a = dist[q[1] * s.w + q[0]];
            let b = dist[pick[1] * s.w + pick[0]];
            if (scared && a > b) || (!scared && a < b) { pick = q; }
        }
    }
    s.sentries[k].x = pick[0];
    s.sentries[k].y = pick[1];
    s
}

fn any_shield(s) {
    for r in s.runners { if r.alive && r.shield > 0 { return true; } }
    false
}

fn collide(s, k, old, from) {
    for p in 0..s.n {
        let r = s.runners[p];
        let t = s.sentries[k];
        // A sentry that waits at its home is inactive: no points and no harm.
        if !r.alive || t.wait > 0 { continue; }
        let same = r.x == t.x && r.y == t.y;
        let swapped = r.x == old[0] && r.y == old[1] && from[p][0] == t.x && from[p][1] == t.y;
        if !same && !swapped { continue; }
        if r.shield > 0 {
            s.runners[p].score += 10;
            s.sentries[k].x = t.hx;
            s.sentries[k].y = t.hy;
            s.sentries[k].wait = 12;
        } else {
            s.runners[p].lives -= 1;
            if s.runners[p].lives <= 0 {
                s.runners[p].alive = false;
                s.runners[p].why = "被哨兵抓住";
            } else {
                s.runners[p].x = r.sx;
                s.runners[p].y = r.sy;
                s.runners[p].shield = 5;
            }
        }
    }
    s
}

fn outcome(s, forced) {
    let k = active(s).len();
    if !forced && k > 0 && s.left > 0 { return; }
    let ranks = [];
    let scores = [];
    for p in 0..s.n {
        scores.push(s.runners[p].score.to_float());
        let r = 1;
        for q in 0..s.n { if s.runners[q].score > s.runners[p].score { r += 1; } }
        ranks.push(r);
    }
    #{ ranks: ranks, scores: scores, reason: if s.left == 0 { "光点收集完毕" } else if k == 0 { "全部出局" } else { "回合用完" } }
}

fn view(s) {
    let entities = [];
    let players = [];
    let scared = any_shield(s);
    for t in s.sentries { entities.push(#{ type: if scared { "sentry_scared" } else { "sentry" }, x: t.x, y: t.y }); }
    for p in 0..s.n {
        let r = s.runners[p];
        players.push(#{ score: r.score, alive: r.alive, lives: r.lives, note: if r.why != "" { r.why } else if r.shield > 0 { `护盾 ${r.shield}` } else { "" } });
        if r.alive { entities.push(#{ type: if r.shield > 0 { "shielded" } else { "runner" }, player: p, x: r.x, y: r.y }); }
    }
    #{ cells: s.cells, entities: entities, players: players }
}
