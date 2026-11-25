//! Misbehaving bots must never hang or break a match. Needs Python and gcc.
use npe_core::*;
use npe_runner::*;
use std::path::PathBuf;
use std::time::{Duration, Instant};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn bot(name: &str, code: &str) -> BotSpec {
    let dir = root().join("target/hostile");
    std::fs::create_dir_all(&dir).unwrap();
    let f = dir.join(name);
    std::fs::write(&f, code).unwrap();
    let tk = Toolkit { sdks: root().join("sdks"), cache: root().join("target/hostile/cache") };
    BotSpec { name: name.into(), lang: lang_of(&f).unwrap().into(), command: build(&tk, &f).unwrap() }
}

const GOOD: &str = "import npe\nnpe.run(lambda o: {'dir': 4})\n";

/// Play the bad bot against a bot that stands still. The good bot must not be blamed.
fn versus(name: &str, code: &str) -> Replay {
    let dir = root().join("games/maze");
    let mut spec = GameSpec::parse(&std::fs::read_to_string(dir.join("game.toml")).unwrap()).unwrap();
    spec.max_turns = 40;
    spec.init_timeout_ms = 4000;
    let rules = std::fs::read_to_string(dir.join("rules.rhai")).unwrap();
    let t = Instant::now();
    let r = run_match(&spec, &rules, 1, &[bot(name, code), bot(&format!("good_{name}").replace(".c", ".py"), GOOD)]).unwrap();
    assert!(t.elapsed() < Duration::from_secs(30), "{name} took {:?}", t.elapsed());
    for e in r.frames.iter().flat_map(|f| &f.events) {
        assert_eq!(e.player, 0, "{name}: {e:?}");
    }
    assert!(r.outcome.is_some());
    r
}

fn last_event(r: &Replay) -> Event {
    r.frames.iter().flat_map(|f| &f.events).last().expect("a fault").clone()
}

#[test]
fn infinite_loop_is_a_timeout_and_then_out() {
    let e = last_event(&versus("loop.py", "import npe\ndef t(o):\n    while True: pass\nnpe.run(t)\n"));
    assert!(e.kind == FaultKind::Timeout && e.eliminated);
}

#[test]
fn silent_start_is_out() {
    assert!(last_event(&versus("silent.py", "import time\ntime.sleep(60)\n")).eliminated);
}

#[test]
fn exception_is_a_crash_with_the_traceback() {
    let e = last_event(&versus("boom.py", "import npe\ndef t(o): return 1 // 0\nnpe.run(t)\n"));
    assert!(e.kind == FaultKind::Crash && e.detail.contains("ZeroDivisionError"), "{}", e.detail);
}

#[test]
fn stdout_flood_is_out() {
    assert!(last_event(&versus("flood.py", "import sys\nwhile True: sys.stdout.write('x' * 4096 + '\\n')\n")).eliminated);
}

#[test]
fn one_endless_line_is_out() {
    assert!(last_event(&versus("line.py", "import sys\nwhile True: sys.stdout.write('x' * 4096)\n")).eliminated);
}

#[test]
fn stderr_flood_is_capped() {
    let r = versus("log.py", "import npe\ndef t(o):\n    print('y' * 100000)\n    return {'dir': 4}\nnpe.run(t)\n");
    let total: usize = r.logs.iter().map(|l| l.text.len()).sum();
    assert!(total > 0 && total < 100_000, "{total}");
}

#[test]
fn never_reading_stdin_does_not_block_the_host() {
    let e = last_event(&versus("deaf.py", "import time\nprint('{\"ready\":true}', flush=True)\ntime.sleep(60)\n"));
    assert!(e.eliminated);
}

#[test]
fn late_replies_do_not_answer_the_next_turn() {
    let code = "import npe, time\ndef t(o):\n    if o['turn'] == 0: time.sleep(0.45)\n    return {'dir': 4}\nnpe.run(t)\n";
    let r = versus("late.py", code);
    assert!(r.frames.iter().flat_map(|f| &f.events).all(|e| e.kind == FaultKind::Timeout && !e.eliminated));
}

#[test]
fn c_segfault_is_reported_and_printf_is_harmless() {
    let code = "#include \"maze.h\"\nint think(const State *s) {\n  printf(\"debug %d\\n\", s->turn);\n  if (s->turn == 3) { volatile int *p = 0; *p = 1; }\n  return STAY;\n}\n";
    let r = versus("segv.c", code);
    let e = last_event(&r);
    assert!(e.kind == FaultKind::Crash && e.detail.contains("segmentation fault"), "{}", e.detail);
    assert!(r.logs.iter().any(|l| l.text.contains("debug 2")));
}

#[test]
fn a_changed_header_triggers_a_rebuild() {
    let dir = root().join("target/hostile");
    std::fs::create_dir_all(&dir).unwrap();
    let code = "#include \"maze.h\"\n#include \"mine.h\"\nint think(const State *s) { printf(\"V%d\\n\", V); return STAY; }\n";
    for v in ["1", "2"] {
        std::fs::write(dir.join("mine.h"), format!("#define V {v}\n")).unwrap();
        let r = versus("header.c", code);
        assert!(r.logs.iter().any(|l| l.text == format!("V{v}")));
    }
}

#[test]
fn host_secrets_are_not_visible_to_bots() {
    std::env::set_var("NPE_TEST_SECRET_TOKEN", "hunter2");
    let code = "import npe, os\ndef t(o):\n    print('SECRET', os.environ.get('NPE_TEST_SECRET_TOKEN'))\n    return {'dir': 4}\nnpe.run(t)\n";
    let r = versus("env.py", code);
    assert!(r.logs.iter().any(|l| l.text == "SECRET None"));
}
