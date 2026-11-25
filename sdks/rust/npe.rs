//! NPlayEngine Rust SDK. Single file, std only.
//!
//! ```ignore
//! use npe::Json;
//! fn think(obs: &Json) -> String { npe::action(&[("dir", 1)]) }
//! npe::bot!(think);
//! ```

#[derive(Clone, Debug, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

static NULL: Json = Json::Null;

impl Json {
    /// Path lookup, for example `get("snakes[2].body[0][1]")`.
    pub fn get(&self, path: &str) -> &Json {
        let path = path.replace('[', ".").replace(']', "");
        let mut cur = self;
        for part in path.split('.').filter(|p| !p.is_empty()) {
            cur = match cur {
                Json::Obj(o) => o.iter().find(|(k, _)| k == part).map_or(&NULL, |x| &x.1),
                Json::Arr(a) => part.parse::<usize>().ok().and_then(|i| a.get(i)).unwrap_or(&NULL),
                _ => &NULL,
            };
        }
        cur
    }
    pub fn int(&self, path: &str) -> i32 {
        self.get(path).as_int()
    }
    pub fn as_int(&self) -> i32 {
        if let Json::Num(n) = self { *n as i32 } else { 0 }
    }
    pub fn bool(&self, path: &str) -> bool {
        self.get(path) == &Json::Bool(true)
    }
    pub fn items(&self) -> &[Json] {
        if let Json::Arr(a) = self { a } else { &[] }
    }
    pub fn grid(&self, path: &str) -> Vec<Vec<i32>> {
        self.get(path).items().iter().map(|r| r.items().iter().map(Json::as_int).collect()).collect()
    }
    pub fn parse(s: &str) -> Json {
        value(s.as_bytes(), &mut 0)
    }
}

fn ws(s: &[u8], i: &mut usize) {
    while *i < s.len() && b" \t\r\n,:".contains(&s[*i]) {
        *i += 1;
    }
}

fn value(s: &[u8], i: &mut usize) -> Json {
    ws(s, i);
    match s.get(*i) {
        Some(b'{') => {
            let mut o = vec![];
            *i += 1;
            ws(s, i);
            while *i < s.len() && s[*i] != b'}' {
                let k = string(s, i);
                o.push((k, value(s, i)));
                ws(s, i);
            }
            *i += 1;
            Json::Obj(o)
        }
        Some(b'[') => {
            let mut a = vec![];
            *i += 1;
            ws(s, i);
            while *i < s.len() && s[*i] != b']' {
                a.push(value(s, i));
                ws(s, i);
            }
            *i += 1;
            Json::Arr(a)
        }
        Some(b'"') => Json::Str(string(s, i)),
        _ => {
            let b = *i;
            while *i < s.len() && !b",]} \r\n".contains(&s[*i]) {
                *i += 1;
            }
            match &s[b..*i] {
                b"true" => Json::Bool(true),
                b"false" => Json::Bool(false),
                t => std::str::from_utf8(t).ok().and_then(|t| t.parse().ok()).map_or(Json::Null, Json::Num),
            }
        }
    }
}

// Escapes other than \" and \\ do not occur in this protocol.
fn string(s: &[u8], i: &mut usize) -> String {
    let mut b = vec![];
    *i += 1;
    while *i < s.len() && s[*i] != b'"' {
        if s[*i] == b'\\' {
            *i += 1;
        }
        b.push(*s.get(*i).unwrap_or(&b' '));
        *i += 1;
    }
    *i += 1;
    String::from_utf8_lossy(&b).into_owned()
}

/// Build an action object from integer fields.
pub fn action(fields: &[(&str, i32)]) -> String {
    let f: Vec<String> = fields.iter().map(|(k, v)| format!("\"{k}\":{v}")).collect();
    format!("{{{}}}", f.join(","))
}

/// One protocol line in, at most one reply out.
pub fn handle(line: &str, think: &mut dyn FnMut(&Json) -> String) -> Option<String> {
    let m = Json::parse(line);
    match m.get("type") {
        Json::Str(t) if t == "init" => Some("{\"ready\":true}".into()),
        Json::Str(t) if t == "turn" => {
            Some(format!("{{\"turn\":{},\"action\":{}}}", m.int("turn"), think(m.get("obs"))))
        }
        _ => None,
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn run(mut think: impl FnMut(&Json) -> String) {
    use std::io::{BufRead, Write};
    let mut wire = wire::take();
    for line in std::io::stdin().lock().lines().map_while(Result::ok) {
        if let Some(r) = handle(&line, &mut think) {
            let _ = wire.write_all((r + "\n").as_bytes());
            let _ = wire.flush();
        }
    }
}

/// Moves the real stdout away, so println! lands in the debug log.
#[cfg(unix)]
mod wire {
    use std::os::fd::FromRawFd;
    extern "C" {
        fn dup(fd: i32) -> i32;
        fn dup2(a: i32, b: i32) -> i32;
    }
    pub fn take() -> std::fs::File {
        unsafe {
            let f = std::fs::File::from_raw_fd(dup(1));
            dup2(2, 1);
            f
        }
    }
}
#[cfg(windows)]
mod wire {
    use std::os::windows::io::{FromRawHandle, RawHandle};
    extern "system" {
        fn GetStdHandle(n: u32) -> RawHandle;
        fn SetStdHandle(n: u32, h: RawHandle) -> i32;
    }
    const OUT: u32 = -11i32 as u32;
    const ERR: u32 = -12i32 as u32;
    pub fn take() -> std::fs::File {
        unsafe {
            let f = std::fs::File::from_raw_handle(GetStdHandle(OUT));
            SetStdHandle(OUT, GetStdHandle(ERR));
            f
        }
    }
}

/// Defines `main`, plus the browser exports on wasm32.
#[macro_export]
macro_rules! bot {
    ($think:ident) => {
        #[cfg(not(target_arch = "wasm32"))]
        fn main() {
            $crate::run($think);
        }
        #[cfg(target_arch = "wasm32")]
        mod npe_exports {
            use std::cell::RefCell;
            thread_local! {
                static IN: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
                static OUT: RefCell<Vec<u8>> = const { RefCell::new(Vec::new()) };
            }
            #[no_mangle]
            pub extern "C" fn npe_buf(len: usize) -> *mut u8 {
                IN.with_borrow_mut(|b| {
                    b.clear();
                    b.resize(len, 0);
                    b.as_mut_ptr()
                })
            }
            #[no_mangle]
            pub extern "C" fn npe_line() -> *const u8 {
                let line = IN.with_borrow(|b| String::from_utf8_lossy(b).into_owned());
                let mut r = $crate::handle(&line, &mut super::$think).unwrap_or_default().into_bytes();
                r.push(0);
                OUT.with_borrow_mut(|o| {
                    *o = r;
                    o.as_ptr()
                })
            }
        }
    };
}
