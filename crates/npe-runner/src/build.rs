use process_wrap::std::*;
use std::collections::HashMap;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const BUILD_LIMIT: Duration = Duration::from_secs(60);

/// How to start a bot that has been compiled.
#[derive(Clone, Debug)]
pub struct BotCommand {
    pub program: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub dir: PathBuf,
}

/// Where the SDKs live and where build products go.
#[derive(Clone)]
pub struct Toolkit {
    pub sdks: PathBuf,
    pub cache: PathBuf,
}

pub fn lang_of(path: &Path) -> Option<&'static str> {
    Some(match path.extension()?.to_str()? {
        "c" => "c",
        "cpp" | "cc" | "cxx" => "cpp",
        "py" => "python",
        "java" => "java",
        "rs" => "rust",
        "js" | "mjs" => "js",
        "exe" => "native",
        _ => return None,
    })
}

/// Executables on Unix usually have no extension.
fn lang_or_native(path: &Path) -> Option<&'static str> {
    lang_of(path).or_else(|| path.extension().is_none().then_some("native"))
}

// Two threads must not compile the same bot at once. Different bots may.
static LOCKS: Mutex<Option<HashMap<String, Arc<Mutex<()>>>>> = Mutex::new(None);

fn lock_for(key: &str) -> Arc<Mutex<()>> {
    let mut g = LOCKS.lock().unwrap_or_else(|e| e.into_inner());
    g.get_or_insert_default().entry(key.into()).or_default().clone()
}

fn fnv(parts: &[&[u8]]) -> String {
    let mut h = 0xcbf2_9ce4_8422_2325u64;
    for p in parts {
        for b in p.iter().chain(&[0u8]) {
            h = (h ^ u64::from(*b)).wrapping_mul(0x100_0000_01b3);
        }
    }
    format!("{h:016x}")
}

fn first_that_runs(names: &[&str], flag: &str) -> Option<String> {
    names.iter().find(|n| Command::new(n).arg(flag).output().is_ok_and(|o| o.status.success())).map(|s| s.to_string())
}

fn run(cmd: &mut Command, what: &str) -> Result<(), String> {
    let mut owned = Command::new(cmd.get_program());
    owned.args(cmd.get_args()).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut w = StdCommandWrap::from(owned);
    #[cfg(windows)]
    w.wrap(JobObject);
    #[cfg(unix)]
    w.wrap(ProcessGroup::leader());
    let mut child = w.spawn().map_err(|e| format!("{what}: cannot start the compiler: {e}"))?;
    // Keep the first part of the output and drain the rest.
    let drain = |mut r: Box<dyn Read + Send>| {
        std::thread::spawn(move || {
            let mut keep = vec![];
            let _ = r.by_ref().take(16 * 1024).read_to_end(&mut keep);
            let _ = std::io::copy(&mut r, &mut std::io::sink());
            keep
        })
    };
    let err = drain(Box::new(child.stderr().take().unwrap()));
    let out = drain(Box::new(child.stdout().take().unwrap()));
    let start = Instant::now();
    let status = loop {
        match child.try_wait().map_err(|e| e.to_string())? {
            Some(s) => break s,
            None if start.elapsed() > BUILD_LIMIT => {
                let _ = child.start_kill();
                let _ = child.wait();
                return Err(format!("{what}: compiling took more than {} s", BUILD_LIMIT.as_secs()));
            }
            None => std::thread::sleep(Duration::from_millis(10)),
        }
    };
    if status.success() {
        return Ok(());
    }
    let (err, out) = (err.join().unwrap_or_default(), out.join().unwrap_or_default());
    let text = format!("{}{}", String::from_utf8_lossy(&err), String::from_utf8_lossy(&out));
    let cut: String = text.chars().take(6000).collect();
    Err(format!("{what} failed to compile\n{cut}"))
}

/// Contents of every file in `dir` that `want` accepts, in a stable order.
fn contents(dir: &Path, want: impl Fn(&Path) -> bool) -> Vec<u8> {
    let mut files: Vec<_> = std::fs::read_dir(dir).into_iter().flatten().flatten().map(|e| e.path()).collect();
    files.retain(|p| p.is_file() && want(p));
    files.sort();
    files.into_iter().flat_map(|f| std::fs::read(f).unwrap_or_default()).collect()
}

fn exe(p: PathBuf) -> PathBuf {
    if cfg!(windows) {
        p.with_extension("exe")
    } else {
        p
    }
}

fn s(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

/// Compile `src` when needed and return the command that runs it.
pub fn build(tk: &Toolkit, src: &Path) -> Result<BotCommand, String> {
    let src = std::path::absolute(src).map_err(|e| e.to_string())?;
    let name = src.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let lang = lang_or_native(&src).ok_or_else(|| format!("{name}: unknown language"))?;
    let code = std::fs::read(&src).map_err(|e| format!("{}: {e}", src.display()))?;
    let sdk = tk.sdks.join(lang);
    if lang == "native" {
        return Ok(BotCommand { program: s(&src), args: vec![], env: vec![], dir: src.parent().unwrap_or(Path::new(".")).into() });
    }
    let mut deps = contents(&sdk, |_| true);
    if lang == "c" || lang == "cpp" {
        // C++ bots use the C headers, and bots may include headers of their own.
        deps.extend(contents(&tk.sdks.join("c"), |_| true));
        let header = |p: &Path| p.extension().is_some_and(|e| e == "h" || e == "hpp");
        deps.extend(contents(src.parent().unwrap_or(Path::new(".")), header));
    }
    let key = fnv(&[lang.as_bytes(), s(&src).as_bytes(), &code, &deps]);
    let lock = lock_for(&key);
    let _g = lock.lock().unwrap_or_else(|e| e.into_inner());
    let out = tk.cache.join(key);
    std::fs::create_dir_all(&out).map_err(|e| e.to_string())?;
    let done = out.join(".done");
    let fresh = done.exists();
    let finish = || std::fs::write(&done, b"").map_err(|e| e.to_string());
    let mut cmd = BotCommand { program: String::new(), args: vec![], env: vec![], dir: out.clone() };
    match lang {
        "c" | "cpp" => {
            let bin = exe(out.join("bot"));
            if !fresh {
                let (cands, std): (&[&str], &str) =
                    if lang == "c" { (&["gcc", "clang", "cc"], "-std=c11") } else { (&["g++", "clang++", "c++"], "-std=c++17") };
                let cc = first_that_runs(cands, "--version").ok_or("no C/C++ compiler found (install gcc or clang)")?;
                let mut c = Command::new(cc);
                c.args([std, "-O2", "-g", "-o"]).arg(&bin).arg(&src).arg("-I").arg(tk.sdks.join("c"));
                if cfg!(windows) {
                    c.arg("-static");
                }
                run(c.arg("-lm"), &name)?;
                finish()?;
            }
            cmd.program = s(&bin);
        }
        "rust" => {
            let bin = exe(out.join("bot"));
            if !fresh {
                let lib = out.join("libnpe.rlib");
                let mut c = Command::new("rustc");
                c.args(["--edition=2021", "-O", "--crate-type=rlib", "--crate-name=npe", "-o"]).arg(&lib).arg(sdk.join("npe.rs"));
                run(&mut c, "Rust SDK")?;
                let mut c = Command::new("rustc");
                c.args(["--edition=2021", "-O", "--crate-name=bot", "-o"]).arg(&bin).arg(&src);
                c.arg("--extern").arg(format!("npe={}", lib.display()));
                run(&mut c, &name)?;
                finish()?;
            }
            cmd.program = s(&bin);
        }
        "java" => {
            let main = src.file_stem().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            if !fresh {
                let mut c = Command::new("javac");
                c.args(["-encoding", "UTF-8", "-d"]).arg(&out).arg(sdk.join("Npe.java")).arg(&src);
                run(&mut c, &name)?;
                finish()?;
            }
            cmd.program = "java".into();
            cmd.args = ["-Xss64m", "-Xmx256m", "-XX:+UseSerialGC", "-Dfile.encoding=UTF-8", "-cp"].map(String::from).into();
            cmd.args.extend([s(&out), main]);
        }
        "python" => {
            cmd.program = first_that_runs(&["python", "python3", "py"], "--version").ok_or("no Python found")?;
            cmd.args = vec!["-B".into(), "-u".into(), s(&src)];
            cmd.env = vec![("PYTHONPATH".into(), s(&sdk)), ("PYTHONIOENCODING".into(), "utf-8".into())];
        }
        "js" => {
            cmd.program = "node".into();
            cmd.args = vec![s(&sdk.join("npe.js")), s(&src)];
        }
        _ => unreachable!("native bots return early"),
    }
    Ok(cmd)
}
