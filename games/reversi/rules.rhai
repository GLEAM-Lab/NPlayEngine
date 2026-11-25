// Reversi. Cells: 0 empty, 1 player 0, 2 player 1. Index = y * n + x.

fn init(ctx) {
    let n = ctx.config.size;
    let board = [];
    board.pad(n * n, 0);
    let m = n / 2;
    board[(m - 1) * n + m - 1] = 2;
    board[m * n + m] = 2;
    board[(m - 1) * n + m] = 1;
    board[m * n + m - 1] = 1;
    let s = #{ n: n, board: board, cur: 0, last: (), flips: [], moves: [], over: false, out: -1, passed: false };
    s.moves = legal(s, 0);
    s
}

fn active(s) { if s.over { [] } else { [s.cur] } }

// Indices flipped when color c plays at (x, y).
fn flips(s, x, y, c) {
    let all = [];
    let n = s.n;
    for d in [[1, 0], [1, 1], [0, 1], [-1, 1], [-1, 0], [-1, -1], [0, -1], [1, -1]] {
        let line = [];
        let i = x + d[0];
        let j = y + d[1];
        while i >= 0 && j >= 0 && i < n && j < n && s.board[j * n + i] == 3 - c {
            line.push(j * n + i);
            i += d[0];
            j += d[1];
        }
        if line.len() > 0 && i >= 0 && j >= 0 && i < n && j < n && s.board[j * n + i] == c {
            all.append(line);
        }
    }
    all
}

fn legal(s, p) {
    let r = [];
    for y in 0..s.n {
        for x in 0..s.n {
            if s.board[y * s.n + x] == 0 && flips(s, x, y, p + 1).len() > 0 { r.push(#{ x: x, y: y }); }
        }
    }
    r
}

fn count(s, c) {
    let k = 0;
    for v in s.board { if v == c { k += 1; } }
    k
}

fn observe(s, p) {
    let rows = [];
    for y in 0..s.n { rows.push(s.board.extract(y * s.n, s.n)); }
    #{ size: s.n, me: p + 1, opponent: 2 - p, board: rows, moves: s.moves, last: s.last }
}

fn validate(s, p, a) {
    if type_of(a) != "map" { return "action must be an object {x, y}"; }
    if type_of(a.x) != "i64" || type_of(a.y) != "i64" { return "x and y must be integers"; }
    for m in s.moves { if m.x == a.x && m.y == a.y { return ""; } }
    `(${a.x}, ${a.y}) is not a legal move`
}

fn default_action(s, p) { s.moves[0] }

fn eliminate(s, p) {
    s.out = p;
    s.over = true;
    s
}

fn apply(s, actions) {
    let p = s.cur;
    let a = actions[p];
    let f = flips(s, a.x, a.y, p + 1);
    for i in f { s.board[i] = p + 1; }
    s.board[a.y * s.n + a.x] = p + 1;
    s.last = #{ x: a.x, y: a.y };
    s.flips = f;
    s.passed = false;
    let other = legal(s, 1 - p);
    if other.len() > 0 {
        s.cur = 1 - p;
        s.moves = other;
    } else {
        s.passed = true;
        s.moves = legal(s, p);
        if s.moves.len() == 0 { s.over = true; }
    }
    s
}

fn outcome(s, forced) {
    if !s.over && !forced { return; }
    let a = count(s, 1).to_float();
    let b = count(s, 2).to_float();
    if s.out == 0 { return #{ ranks: [2, 1], scores: [a, b], reason: "黑方出局" }; }
    if s.out == 1 { return #{ ranks: [1, 2], scores: [a, b], reason: "白方出局" }; }
    if a > b { return #{ ranks: [1, 2], scores: [a, b], reason: "黑方子多" }; }
    if b > a { return #{ ranks: [2, 1], scores: [a, b], reason: "白方子多" }; }
    #{ ranks: [1, 1], scores: [a, b], reason: "平局" }
}

fn view(s) {
    let players = [];
    for p in 0..2 {
        players.push(#{ score: count(s, p + 1), alive: s.out != p, active: !s.over && s.cur == p, note: if s.passed && !s.over && s.cur != p { "无处可下" } else { "" } });
    }
    let marks = if s.last == () { [] } else { [#{ x: s.last.x, y: s.last.y }] };
    #{ cells: s.board, entities: [], marks: marks, hints: if s.over { [] } else { s.moves }, players: players }
}
