//! Browser binding with a plain C ABI, so no wasm-bindgen step is needed.
//! JS writes a JSON request into memory from `npe_alloc` and calls `npe_call`,
//! which returns `(ptr << 32) | len` of a JSON reply. JS frees both buffers.

use npe_core::{GameSpec, Match, PlayerId, PlayerInfo, Reply};
use serde::Deserialize;
use serde_json::{json, Value};
use std::cell::RefCell;
use std::collections::BTreeMap;

thread_local! {
    static MATCHES: RefCell<(u32, BTreeMap<u32, Match>)> = const { RefCell::new((0, BTreeMap::new())) };
}

#[derive(Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
enum Request {
    Spec { toml: String },
    New { toml: String, script: String, seed: u32, players: Vec<PlayerInfo> },
    Step { id: u32, replies: BTreeMap<String, Reply> },
    Replay { id: u32 },
    Free { id: u32 },
}

fn status(id: u32, m: &Match) -> Result<Value, String> {
    let active = m.active()?;
    let mut msgs = serde_json::Map::new();
    for p in &active {
        msgs.insert(p.to_string(), m.turn_message(*p)?);
    }
    Ok(json!({
        "id": id, "turn": m.turn(), "active": active, "messages": msgs,
        "frame": m.last_frame(), "outcome": m.outcome(),
    }))
}

fn handle(req: Request) -> Result<Value, String> {
    MATCHES.with_borrow_mut(|(next, map)| match req {
        Request::Spec { toml } => {
            serde_json::to_value(GameSpec::parse(&toml)?).map_err(|e| e.to_string())
        }
        Request::New { toml, script, seed, players } => {
            let n = players.len();
            let m = Match::new(GameSpec::parse(&toml)?, &script, seed, players)?;
            *next += 1;
            let mut s = status(*next, &m)?;
            s["init"] = (0..n).map(|p| m.init_message(p)).collect();
            map.insert(*next, m);
            Ok(s)
        }
        Request::Step { id, replies } => {
            let m = map.get_mut(&id).ok_or("unknown match")?;
            // JSON object keys stay strings inside a tagged enum, so parse them here.
            let replies = replies
                .into_iter()
                .map(|(k, r)| k.parse::<PlayerId>().map(|k| (k, r)).map_err(|e| e.to_string()))
                .collect::<Result<_, _>>()?;
            m.step(replies)?;
            status(id, m)
        }
        Request::Replay { id } => {
            let m = map.get(&id).ok_or("unknown match")?;
            serde_json::to_value(m.replay()).map_err(|e| e.to_string())
        }
        Request::Free { id } => {
            map.remove(&id);
            Ok(Value::Null)
        }
    })
}

#[no_mangle]
pub extern "C" fn npe_alloc(len: usize) -> *mut u8 {
    Box::into_raw(vec![0u8; len].into_boxed_slice()) as *mut u8
}

/// # Safety
/// `ptr` and `len` must come from `npe_alloc` or `npe_call`, and be freed once.
#[no_mangle]
pub unsafe extern "C" fn npe_free(ptr: *mut u8, len: usize) {
    drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(ptr, len)));
}

/// # Safety
/// `ptr..ptr+len` must be a buffer from `npe_alloc`.
#[no_mangle]
pub unsafe extern "C" fn npe_call(ptr: *const u8, len: usize) -> u64 {
    let bytes = std::slice::from_raw_parts(ptr, len);
    let res = serde_json::from_slice::<Request>(bytes)
        .map_err(|e| e.to_string())
        .and_then(handle);
    let out = match res {
        Ok(v) => json!({"ok": v}),
        Err(e) => json!({"err": e}),
    };
    let b = out.to_string().into_bytes().into_boxed_slice();
    let len = b.len() as u64;
    ((Box::into_raw(b) as *mut u8 as usize as u64) << 32) | len
}
