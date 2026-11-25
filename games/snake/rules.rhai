// Simultaneous snakes. dir: 0 up, 1 right, 2 down, 3 left. Body[0] is the head.

fn init(ctx) {
    let c = ctx.config;
    let s = #{
        w: c.width, h: c.height, want_food: c.food, n: ctx.players,
        rng: ctx.seed, turn: 0, food: [], snakes: []
    };
    // Spawn points sit on a 3 x 2 layout, so no seat is boxed in by another one.
    let spots = [[0, 0], [2, 1], [2, 0], [0, 1], [1, 0], [1, 1]];
    for p in 0..s.n {
        let cx = s.w * (2 * spots[p][0] + 1) / 6;
        let cy = s.h * (2 * spots[p][1] + 1) / 4;
        let dir = if spots[p][1] == 0 { 2 } else { 0 };
        let body = [];
        for k in 0..c.start_length {
            body.push([cx, if dir == 2 { cy - k } else { cy + k }]);
        }
        s.snakes.push(#{ body: body, dir: dir, alive: true, died: -1, eaten: 0, why: "" });
    }
    s = fill_food(s);
    s
}

fn occupied(s, x, y) {
    for q in s.snakes {
        if !q.alive { continue; }
        for b in q.body { if b[0] == x && b[1] == y { return true; } }
    }
    for f in s.food { if f[0] == x && f[1] == y { return true; } }
    false
}

fn fill_food(s) {
    let tries = 0;
    while s.food.len() < s.want_food && tries < 60 {
        tries += 1;
        s.rng = rng_next(s.rng);
        let i = s.rng % (s.w * s.h);
        let x = i % s.w;
        let y = i / s.w;
        if !occupied(s, x, y) { s.food.push([x, y]); }
    }
    s
}

fn active(s) {
    let r = [];
    for p in 0..s.n { if s.snakes[p].alive { r.push(p); } }
    r
}

fn observe(s, p) {
    let snakes = [];
    for q in s.snakes {
        snakes.push(#{ alive: q.alive, dir: q.dir, length: q.body.len(), body: if q.alive { q.body } else { [] } });
    }
    #{ width: s.w, height: s.h, me: p, turn: s.turn, snakes: snakes, food: s.food }
}

fn validate(s, p, a) {
    if type_of(a) != "map" || type_of(a.dir) != "i64" { return "action must be {dir} with an integer"; }
    if a.dir < 0 || a.dir > 3 { return `dir must be 0 to 3, got ${a.dir}`; }
    ""
}

fn default_action(s, p) { #{ dir: s.snakes[p].dir } }

fn eliminate(s, p) {
    s.snakes[p].alive = false;
    s.snakes[p].died = s.turn;
    s.snakes[p].why = "程序出局";
    s
}

fn apply(s, actions) {
    s.turn += 1;
    let heads = [];
    let grows = [];
    for p in 0..s.n {
        let q = s.snakes[p];
        if !q.alive || actions[p] == () { heads.push(()); grows.push(false); continue; }
        let d = actions[p].dir;
        // A snake cannot reverse onto its own neck. It keeps going straight.
        if (d + 2) % 4 == q.dir { d = q.dir; }
        s.snakes[p].dir = d;
        let x = q.body[0][0];
        let y = q.body[0][1];
        if d == 0 { y -= 1; } else if d == 1 { x += 1; } else if d == 2 { y += 1; } else { x -= 1; }
        heads.push([x, y]);
        let g = false;
        for f in s.food { if f[0] == x && f[1] == y { g = true; } }
        grows.push(g);
    }
    // Tails leave before heads arrive, so following a tail is safe.
    for p in 0..s.n {
        if heads[p] != () && !grows[p] { s.snakes[p].body.pop(); }
    }
    let dead = [];
    for p in 0..s.n {
        let h = heads[p];
        if h == () { dead.push(""); continue; }
        let why = "";
        if h[0] < 0 || h[1] < 0 || h[0] >= s.w || h[1] >= s.h { why = "撞墙"; }
        for q in 0..s.n {
            if !s.snakes[q].alive { continue; }
            for b in s.snakes[q].body {
                if b[0] == h[0] && b[1] == h[1] { why = if q == p { "撞到自己" } else { "撞到别的蛇" }; }
            }
            if q != p && heads[q] != () && heads[q][0] == h[0] && heads[q][1] == h[1] { why = "头碰头"; }
        }
        dead.push(why);
    }
    for p in 0..s.n {
        let h = heads[p];
        if h == () { continue; }
        if dead[p] != "" {
            s.snakes[p].alive = false;
            s.snakes[p].died = s.turn;
            s.snakes[p].why = dead[p];
        } else {
            s.snakes[p].body.insert(0, h);
            if grows[p] {
                s.snakes[p].eaten += 1;
                let left = [];
                for f in s.food { if f[0] != h[0] || f[1] != h[1] { left.push(f); } }
                s.food = left;
            }
        }
    }
    fill_food(s)
}

fn alive_count(s) {
    let k = 0;
    for q in s.snakes { if q.alive { k += 1; } }
    k
}

// Each survived turn is worth 1 point and each fruit 10 points. A living snake
// counts the current turn too, so it always beats one that just dropped out.
fn score(s, p) {
    let q = s.snakes[p];
    let lived = if q.alive { s.turn + 1 } else { q.died };
    lived + 10 * q.eaten
}

fn outcome(s, forced) {
    let k = alive_count(s);
    let over = k == 0 || (s.n > 1 && k <= 1);
    if !forced && !over { return; }
    let scores = [];
    for p in 0..s.n { scores.push(score(s, p)); }
    let ranks = [];
    for p in 0..s.n {
        let r = 1;
        for q in 0..s.n { if scores[q] > scores[p] { r += 1; } }
        ranks.push(r);
    }
    let out = [];
    for v in scores { out.push(v.to_float()); }
    #{ ranks: ranks, scores: out, reason: if forced { "回合用完" } else if k == 1 { "只剩一条蛇" } else { "全部出局" } }
}

fn view(s) {
    let entities = [];
    for f in s.food { entities.push(#{ type: "food", x: f[0], y: f[1] }); }
    let players = [];
    for p in 0..s.n {
        let q = s.snakes[p];
        players.push(#{ score: score(s, p), alive: q.alive, note: q.why });
        if !q.alive { continue; }
        for i in 1..q.body.len() {
            entities.push(#{ type: "body", player: p, x: q.body[i][0], y: q.body[i][1] });
        }
        entities.push(#{ type: "head", player: p, x: q.body[0][0], y: q.body[0][1], angle: q.dir * 90 - 90 });
    }
    #{ entities: entities, players: players }
}
