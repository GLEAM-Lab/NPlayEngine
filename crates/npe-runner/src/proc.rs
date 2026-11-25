use crate::build::BotCommand;
use process_wrap::std::*;
use serde_json::Value;
use std::io::{BufRead, BufReader, Read, Write};
use std::process::Stdio;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{channel, sync_channel, Receiver, RecvTimeoutError, Sender, TrySendError};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

const MAX_LINE: u64 = 64 * 1024;
const MAX_LOG: usize = 64 * 1024;

pub enum Got {
    Line(Value),
    Timeout,
    Gone(String),
}

type Log = Arc<Mutex<(Vec<(u32, String)>, usize)>>;

pub struct Bot {
    child: Box<dyn StdChildWrapper>,
    tx: Sender<Vec<u8>>,
    rx: Receiver<(Instant, Vec<u8>)>,
    log: Log,
    pub turn: Arc<AtomicU32>,
    killed: bool,
}

impl Bot {
    pub fn spawn(cmd: &BotCommand) -> Result<Bot, String> {
        let mut w = StdCommandWrap::with_new(&cmd.program, |c| {
            c.args(&cmd.args).current_dir(&cmd.dir);
            c.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
            // Bots must not see tokens that happen to sit in the host's environment.
            c.env_clear();
            for k in KEEP_ENV {
                if let Some(v) = std::env::var_os(k) {
                    c.env(k, v);
                }
            }
            c.envs(cmd.env.iter().map(|(k, v)| (k, v)));
        });
        #[cfg(windows)]
        w.wrap(JobObject);
        #[cfg(unix)]
        w.wrap(ProcessGroup::leader());
        let mut child = w.spawn().map_err(|e| format!("cannot start {}: {e}", cmd.program))?;
        let mut stdin = child.stdin().take().unwrap();
        let stdout = child.stdout().take().unwrap();
        let stderr = child.stderr().take().unwrap();

        // A bot that stops reading must not block the match, so writes get a thread.
        let (tx, jobs) = channel::<Vec<u8>>();
        std::thread::spawn(move || {
            for b in jobs {
                if stdin.write_all(&b).and_then(|_| stdin.flush()).is_err() {
                    break;
                }
            }
        });

        let (line_tx, rx) = sync_channel(64);
        std::thread::spawn(move || {
            let mut r = BufReader::new(stdout);
            loop {
                let mut buf = vec![];
                match (&mut r).take(MAX_LINE).read_until(b'\n', &mut buf) {
                    Ok(n) if n > 0 && buf.ends_with(b"\n") => {}
                    _ => break,
                }
                // A bot that floods its output is cut off and treated as gone.
                match line_tx.try_send((Instant::now(), buf)) {
                    Ok(()) => {}
                    Err(TrySendError::Full(_) | TrySendError::Disconnected(_)) => break,
                }
            }
        });

        let log: Log = Arc::default();
        let turn = Arc::new(AtomicU32::new(0));
        let (l, t) = (log.clone(), turn.clone());
        std::thread::spawn(move || {
            let mut r = BufReader::new(stderr);
            loop {
                let mut buf = vec![];
                match (&mut r).take(4096).read_until(b'\n', &mut buf) {
                    Ok(n) if n > 0 => {}
                    _ => break,
                }
                let mut g = l.lock().unwrap();
                // Keep draining past the cap so the bot never blocks on stderr.
                if g.1 < MAX_LOG {
                    g.1 += buf.len();
                    let s = String::from_utf8_lossy(&buf).trim_end().to_string();
                    g.0.push((t.load(Ordering::Relaxed), s));
                    if g.1 >= MAX_LOG {
                        g.0.push((t.load(Ordering::Relaxed), "[log limit reached]".into()));
                    }
                }
            }
        });
        Ok(Bot { child, tx, rx, log, turn, killed: false })
    }

    pub fn send(&mut self, v: &Value) -> Instant {
        let mut b = v.to_string().into_bytes();
        b.push(b'\n');
        let _ = self.tx.send(b);
        Instant::now()
    }

    /// Wait for a line accepted by `want`. The arrival time decides a timeout,
    /// not the time at which the host got around to reading the queue.
    pub fn recv(&mut self, since: Instant, limit: Duration, want: impl Fn(&Value) -> bool) -> Got {
        let deadline = since + limit;
        loop {
            match self.rx.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
                Ok((at, line)) => {
                    if at > deadline {
                        return Got::Timeout;
                    }
                    if let Ok(v) = serde_json::from_slice::<Value>(&line) {
                        if want(&v) {
                            return Got::Line(v);
                        }
                    }
                }
                Err(RecvTimeoutError::Timeout) => return Got::Timeout,
                Err(RecvTimeoutError::Disconnected) => return Got::Gone(self.reason()),
            }
        }
    }

    fn reason(&mut self) -> String {
        let mut status = None;
        for _ in 0..20 {
            match self.child.try_wait() {
                Ok(Some(s)) => {
                    status = Some(s);
                    break;
                }
                _ => std::thread::sleep(Duration::from_millis(10)),
            }
        }
        let head = match status {
            Some(s) => describe(s),
            None => "closed or flooded its output".into(),
        };
        let tail: Vec<String> = {
            let g = self.log.lock().unwrap();
            g.0.iter().rev().take(3).rev().map(|x| x.1.clone()).collect()
        };
        if tail.is_empty() {
            head
        } else {
            format!("{head}: {}", tail.join(" | "))
        }
    }

    pub fn kill(&mut self) {
        if !self.killed {
            self.killed = true;
            let _ = self.child.start_kill();
            let _ = self.child.wait();
        }
    }

    pub fn finish(mut self) -> Vec<(u32, String)> {
        self.kill();
        // Give the stderr thread a moment to drain what is already in the pipe.
        std::thread::sleep(Duration::from_millis(20));
        let mut g = self.log.lock().unwrap();
        std::mem::take(&mut g.0)
    }
}

impl Drop for Bot {
    fn drop(&mut self) {
        self.kill();
    }
}

const KEEP_ENV: &[&str] = &[
    "PATH", "SystemRoot", "SYSTEMROOT", "windir", "TEMP", "TMP", "HOME", "USERPROFILE", "LANG", "LC_ALL",
    "JAVA_HOME", "PATHEXT", "COMSPEC", "LOCALAPPDATA", "APPDATA", "NUMBER_OF_PROCESSORS",
];

fn describe(s: std::process::ExitStatus) -> String {
    #[cfg(unix)]
    {
        use std::os::unix::process::ExitStatusExt;
        if let Some(n) = s.signal() {
            return match n {
                11 => "segmentation fault (signal 11)".into(),
                8 => "arithmetic error (signal 8)".into(),
                6 => "abort (signal 6)".into(),
                _ => format!("killed by signal {n}"),
            };
        }
    }
    match s.code().map(|c| c as u32) {
        Some(0xC000_0005) => "segmentation fault (0xC0000005)".into(),
        Some(0xC000_0094) => "integer division by zero (0xC0000094)".into(),
        Some(0xC000_00FD) => "stack overflow (0xC00000FD)".into(),
        Some(0xC000_0374) => "heap corruption (0xC0000374)".into(),
        Some(0) => "exited before the match ended".into(),
        Some(c) => format!("exit code {c}"),
        None => "exited".into(),
    }
}
