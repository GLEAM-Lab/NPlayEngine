mod studio;
mod tournament;

use include_dir::{include_dir, Dir};
use npe_core::{GameSpec, Replay};
use npe_runner::{build, lang_of, BotSpec, Toolkit};
use std::path::{Path, PathBuf};

pub static GAMES: Dir = include_dir!("$CARGO_MANIFEST_DIR/../../games");
static SDKS: Dir = include_dir!("$CARGO_MANIFEST_DIR/../../sdks");
pub static WEB: Dir = include_dir!("$CARGO_MANIFEST_DIR/../../dist/arena");

const HELP: &str = "npe - AI battle engine

  npe games                                 list games
  npe run <game> <bot>... [--seed N] [--replay FILE]
  npe tournament <game> <bot>... [--seats K] [--rounds N] [--jobs N]
  npe studio [--port N]                     local web page with every language

A bot is a source file (.c .cpp .py .java .rs .js) or an executable.
A game is a built-in id or a directory with game.toml and rules.rhai.
Bots run with your own user rights. Only run code that you trust.";

pub struct Game {
    pub spec: GameSpec,
    pub rules: String,
}

pub fn load_game(id: &str) -> Result<Game, String> {
    let dir = Path::new(id);
    let (toml, rules) = if dir.join("game.toml").is_file() {
        let read = |f: &str| std::fs::read_to_string(dir.join(f)).map_err(|e| format!("{f}: {e}"));
        (read("game.toml")?, read("rules.rhai")?)
    } else {
        let get = |f: &str| GAMES.get_file(format!("{id}/{f}")).and_then(|f| f.contents_utf8()).map(String::from);
        let missing = || format!("unknown game '{id}', try: npe games");
        (get("game.toml").ok_or_else(missing)?, get("rules.rhai").ok_or_else(missing)?)
    };
    Ok(Game { spec: GameSpec::parse(&toml)?, rules })
}

/// SDKs are unpacked once per run, because compilers need real files.
pub fn toolkit() -> Result<Toolkit, String> {
    let base = std::env::var_os("LOCALAPPDATA")
        .or_else(|| std::env::var_os("XDG_CACHE_HOME"))
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cache")))
        .ok_or("no cache directory")?
        .join("npe");
    let sdks = base.join(format!("sdks-{}", std::process::id()));
    std::fs::create_dir_all(&sdks).map_err(|e| e.to_string())?;
    SDKS.extract(&sdks).map_err(|e| e.to_string())?;
    Ok(Toolkit { sdks, cache: base.join("build") })
}

pub fn load_bots(tk: &Toolkit, paths: &[String]) -> Result<Vec<BotSpec>, String> {
    paths
        .iter()
        .map(|p| {
            let path = Path::new(p);
            let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            let lang = lang_of(path).unwrap_or("native").to_string();
            Ok(BotSpec { name, lang, command: build(tk, path)? })
        })
        .collect()
}

pub struct Args {
    pub plain: Vec<String>,
    flags: Vec<(String, String)>,
}

impl Args {
    fn parse(it: impl Iterator<Item = String>) -> Result<Args, String> {
        let mut a = Args { plain: vec![], flags: vec![] };
        let mut it = it;
        while let Some(s) = it.next() {
            match s.strip_prefix("--") {
                Some(k) => a.flags.push((k.into(), it.next().ok_or(format!("--{k} needs a value"))?)),
                None => a.plain.push(s),
            }
        }
        Ok(a)
    }
    pub fn text(&self, k: &str) -> Option<&str> {
        self.flags.iter().find(|f| f.0 == k).map(|f| f.1.as_str())
    }
    pub fn num<T: std::str::FromStr>(&self, k: &str, otherwise: T) -> Result<T, String> {
        self.text(k).map_or(Ok(otherwise), |v| v.parse().map_err(|_| format!("--{k}: bad number '{v}'")))
    }
}

fn print_result(r: &Replay) {
    for f in &r.frames {
        for e in &f.events {
            let out = if e.eliminated { ", eliminated" } else { "" };
            println!("  turn {:>3}  {} {:?}{out}: {}", f.turn, r.players[e.player].name, e.kind, e.detail);
        }
    }
    if let Some(o) = &r.outcome {
        println!("{} turns, {}", r.frames.len() - 1, o.reason);
        let mut order: Vec<usize> = (0..r.players.len()).collect();
        order.sort_by_key(|p| o.ranks[*p]);
        for p in order {
            println!("  #{} {:<20} {:<8} score {}", o.ranks[p], r.players[p].name, r.players[p].lang, o.scores[p]);
        }
    }
}

fn main_inner() -> Result<(), String> {
    let mut argv = std::env::args().skip(1);
    let cmd = argv.next().unwrap_or_default();
    let args = Args::parse(argv)?;
    match cmd.as_str() {
        "games" => {
            for d in GAMES.dirs() {
                let g = load_game(&d.path().to_string_lossy())?;
                println!("{:<10} {}-{} players  {}", g.spec.id, g.spec.min_players, g.spec.max_players, g.spec.title);
            }
            Ok(())
        }
        "run" => {
            let (game, bots) = args.plain.split_first().ok_or(HELP)?;
            let g = load_game(game)?;
            let tk = toolkit()?;
            let bots = load_bots(&tk, bots)?;
            let r = npe_runner::run_match(&g.spec, &g.rules, args.num("seed", 1)?, &bots);
            let _ = std::fs::remove_dir_all(&tk.sdks);
            let r = r?;
            print_result(&r);
            if let Some(f) = args.text("replay") {
                std::fs::write(f, serde_json::to_vec(&r).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
                println!("replay written to {f}");
            }
            Ok(())
        }
        "tournament" => tournament::run(&args),
        "studio" => studio::run(args.num("port", 8737)?),
        _ => {
            println!("{HELP}");
            Ok(())
        }
    }
}

fn main() {
    if let Err(e) = main_inner() {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
