// 黑白棋:优先占角,避开角旁边的格子。
use npe::Json;

fn value(m: &Json) -> i32 {
    let edge = |v: i32| v == 0 || v == 7;
    let near = |v: i32| v <= 1 || v >= 6;
    let (x, y) = (m.int("x"), m.int("y"));
    if edge(x) && edge(y) {
        100
    } else if near(x) && near(y) {
        -30
    } else if edge(x) || edge(y) {
        10
    } else {
        1
    }
}

fn think(obs: &Json) -> String {
    let m = obs.get("moves").items().iter().max_by_key(|m| value(m)).unwrap();
    npe::action(&[("x", m.int("x")), ("y", m.int("y"))])
}

npe::bot!(think);
