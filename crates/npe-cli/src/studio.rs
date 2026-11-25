//! Local page that compiles and runs every language natively.
//!
//! It executes code, so other web pages must not be able to reach it:
//! loopback only, exact Host check (DNS rebinding), exact Origin check, and a
//! random token that only the served page knows.

use crate::{load_game, toolkit, GAMES, WEB};
use npe_runner::{build, run_match, BotSpec, Toolkit};
use serde::Deserialize;
use std::io::Read;
use std::sync::Arc;
use tiny_http::{Header, Method, Request, Response, Server};

const MAX_BODY: u64 = 2 * 1024 * 1024;

#[derive(Deserialize)]
struct RunRequest {
    game: String,
    seed: u32,
    bots: Vec<BotSource>,
}

#[derive(Deserialize)]
struct BotSource {
    name: String,
    lang: String,
    code: String,
}

type Reply = Response<std::io::Cursor<Vec<u8>>>;

fn reply(status: u16, mime: &str, body: Vec<u8>) -> Reply {
    let h = |k: &str, v: &str| Header::from_bytes(k, v).expect("static header");
    Response::from_data(body)
        .with_status_code(status)
        .with_header(h("Content-Type", mime))
        .with_header(h("X-Content-Type-Options", "nosniff"))
        .with_header(h("Referrer-Policy", "no-referrer"))
        .with_header(h("Cache-Control", "no-store"))
}

fn error(status: u16, e: &str) -> Reply {
    reply(status, "application/json", serde_json::json!({"error": e}).to_string().into_bytes())
}

fn header<'a>(r: &'a Request, name: &'static str) -> Option<&'a str> {
    r.headers().iter().find(|h| h.field.equiv(name)).map(|h| h.value.as_str())
}

/// Constant-time comparison.
fn same(a: &str, b: &str) -> bool {
    a.len() == b.len() && a.bytes().zip(b.bytes()).fold(0, |x, (p, q)| x | (p ^ q)) == 0
}

/// Java needs the file name to match its public class.
fn file_name(lang: &str, code: &str) -> Result<String, String> {
    Ok(match lang {
        "c" => "bot.c".into(),
        "cpp" => "bot.cpp".into(),
        "python" => "bot.py".into(),
        "rust" => "bot.rs".into(),
        "js" => "bot.js".into(),
        "java" => {
            let name: String = code
                .split("public class ")
                .nth(1)
                .unwrap_or_default()
                .chars()
                .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                .take(64)
                .collect();
            // Reserved device names such as CON or NUL are not valid file names on Windows.
            let reserved = ["con", "prn", "aux", "nul", "npe"].contains(&name.to_lowercase().as_str());
            if name.is_empty() || reserved {
                return Err("Java code needs a valid `public class Name`".into());
            }
            format!("{name}.java")
        }
        _ => return Err("unknown language".into()),
    })
}

fn run_request(tk: &Toolkit, body: &str) -> Result<Vec<u8>, String> {
    let req: RunRequest = serde_json::from_str(body).map_err(|e| e.to_string())?;
    // Built-in games only. A path from the network must never reach the file system.
    if GAMES.get_dir(&req.game).is_none() || req.game.contains(['/', '\\', '.']) {
        return Err("unknown game".into());
    }
    let g = load_game(&req.game)?;
    if req.bots.len() > g.spec.max_players {
        return Err("too many players".into());
    }
    let mut id = [0u8; 8];
    getrandom::fill(&mut id).map_err(|e| e.to_string())?;
    let work = tk.cache.join(format!("studio-{:016x}", u64::from_le_bytes(id)));
    // Build products live inside the work directory, so nothing piles up.
    let tk = Toolkit { sdks: tk.sdks.clone(), cache: work.join("build") };
    let result = (|| {
        let mut bots = vec![];
        for (i, b) in req.bots.iter().enumerate() {
            let dir = work.join(i.to_string());
            std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            let src = dir.join(file_name(&b.lang, &b.code)?);
            std::fs::write(&src, &b.code).map_err(|e| e.to_string())?;
            let command = build(&tk, &src).map_err(|e| format!("player {}: {e}", i + 1))?;
            bots.push(BotSpec { name: b.name.chars().take(40).collect(), lang: b.lang.clone(), command });
        }
        run_match(&g.spec, &g.rules, req.seed, &bots)
    })();
    let _ = std::fs::remove_dir_all(&work);
    serde_json::to_vec(&result?).map_err(|e| e.to_string())
}

fn handle(r: &mut Request, port: u16, token: &str, tk: &Toolkit) -> Reply {
    let hosts = [format!("127.0.0.1:{port}"), format!("localhost:{port}")];
    let host = header(r, "Host").unwrap_or_default();
    if !hosts.iter().any(|h| h == host) {
        return error(403, "bad host");
    }
    let path = r.url().split('?').next().unwrap_or("/").to_string();
    match (r.method(), path.as_str()) {
        (Method::Post, "/api/run") => {
            let origin = header(r, "Origin").unwrap_or_default();
            if origin != format!("http://{host}") || !same(header(r, "X-Npe-Token").unwrap_or_default(), token) {
                return error(403, "forbidden");
            }
            let mut body = String::new();
            let read = r.as_reader().take(MAX_BODY + 1).read_to_string(&mut body);
            if read.is_err() || body.len() as u64 > MAX_BODY {
                return error(413, "request too large");
            }
            match run_request(tk, &body) {
                Ok(b) => reply(200, "application/json", b),
                Err(e) => error(400, &e),
            }
        }
        // Same-origin pages can read this. Cross-origin pages cannot: no CORS header is sent.
        (Method::Get, "/api/token") => {
            if header(r, "Sec-Fetch-Site").is_some_and(|s| s != "same-origin") {
                return error(403, "forbidden");
            }
            reply(200, "application/json", serde_json::json!({"token": token}).to_string().into_bytes())
        }
        (Method::Get, p) => {
            let name = if p == "/" { "index.html" } else { &p[1..] };
            // Lookup in the embedded table, so there is no path traversal.
            let Some(f) = WEB.get_file(name) else { return error(404, "not found") };
            let mime = match name.rsplit('.').next() {
                Some("html") => "text/html; charset=utf-8",
                Some("js") => "text/javascript; charset=utf-8",
                Some("css") => "text/css; charset=utf-8",
                Some("wasm") => "application/wasm",
                Some("json") => "application/json",
                _ => "application/octet-stream",
            };
            reply(200, mime, f.contents().to_vec())
        }
        _ => error(405, "method not allowed"),
    }
}

pub fn run(port: u16) -> Result<(), String> {
    if WEB.get_file("index.html").is_none() {
        return Err("this build has no web page, run: node scripts/build.mjs".into());
    }
    let server = Arc::new(Server::http(("127.0.0.1", port)).map_err(|e| e.to_string())?);
    let port = server.server_addr().to_ip().map_or(port, |a| a.port());
    let mut raw = [0u8; 24];
    getrandom::fill(&mut raw).map_err(|e| e.to_string())?;
    let token: Arc<str> = raw.iter().map(|b| format!("{b:02x}")).collect::<String>().into();
    let tk = Arc::new(toolkit()?);
    println!("open http://127.0.0.1:{port}/   (Ctrl+C to stop)");
    println!("Bots run with your own user rights. Only run code that you trust.");
    let workers: Vec<_> = (0..4)
        .map(|_| {
            let (server, token, tk) = (server.clone(), token.clone(), tk.clone());
            std::thread::spawn(move || {
                while let Ok(mut r) = server.recv() {
                    let out = handle(&mut r, port, &token, &tk);
                    let _ = r.respond(out);
                }
            })
        })
        .collect();
    for w in workers {
        let _ = w.join();
    }
    Ok(())
}
