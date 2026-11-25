// End-to-end and attack tests in a real headless Chrome or Edge. No dependencies.
// Usage: node scripts/e2e.mjs [name filter]
import { spawn } from "node:child_process";
import { createServer } from "node:http";
import { existsSync, mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { extname, join, normalize } from "node:path";
import { fileURLToPath } from "node:url";

const root = process.env.NPE_ROOT ?? fileURLToPath(new URL("../dist/", import.meta.url));
const shots = fileURLToPath(new URL("../target/shots/", import.meta.url));
mkdirSync(shots, { recursive: true });
const MIME = { ".html": "text/html", ".js": "text/javascript", ".css": "text/css", ".wasm": "application/wasm", ".json": "application/json" };
const hits = [];
const server = createServer((q, r) => {
  const p = new URL(q.url, "http://x").pathname;
  hits.push(p);
  const f = join(root, normalize(p));
  if (!f.startsWith(normalize(root)) || !existsSync(f) || !extname(f)) { r.writeHead(404); return r.end(); }
  r.writeHead(200, { "content-type": MIME[extname(f)] ?? "application/octet-stream" });
  r.end(readFileSync(f));
}).listen(0, "127.0.0.1");
await new Promise((r) => server.once("listening", r));
const BASE = process.env.NPE_BASE ?? `http://127.0.0.1:${server.address().port}/arena/index.html`;

const exe = [process.env.CHROME, "C:/Program Files/Google/Chrome/Application/chrome.exe", "C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe", "/usr/bin/google-chrome", "/usr/bin/chromium"].find((p) => p && existsSync(p));
if (!exe) throw new Error("no Chrome or Edge found, set CHROME");
const profile = mkdtempSync(join(tmpdir(), "npe-e2e-"));
const chrome = spawn(exe, ["--headless=new", "--remote-debugging-port=0", `--user-data-dir=${profile}`, "--no-first-run", "--window-size=1440,1000", "about:blank"], { stdio: "ignore" });
let port;
for (let i = 0; i < 100 && !port; i++) {
  await new Promise((r) => setTimeout(r, 200));
  try { port = readFileSync(join(profile, "DevToolsActivePort"), "utf8").split("\n")[0]; } catch {}
}
async function open(url, theme) {
  const t = await (await fetch(`http://127.0.0.1:${port}/json/new?about:blank`, { method: "PUT" })).json();
  const ws = new WebSocket(t.webSocketDebuggerUrl);
  await new Promise((r) => (ws.onopen = r));
  let id = 0;
  const wait = new Map(), errors = [];
  ws.onmessage = (e) => {
    const m = JSON.parse(e.data);
    if (m.id) { wait.get(m.id)(m); wait.delete(m.id); }
    if (m.method === "Runtime.exceptionThrown") errors.push(m.params.exceptionDetails.exception?.description ?? m.params.exceptionDetails.text);
  };
  const cmd = (method, params = {}) => new Promise((r) => { wait.set(++id, r); ws.send(JSON.stringify({ id, method, params })); });
  await cmd("Runtime.enable");
  await cmd("Page.enable");
  if (theme) await cmd("Emulation.setEmulatedMedia", { features: [{ name: "prefers-color-scheme", value: theme }] });
  await cmd("Page.navigate", { url });
  const js = async (f, ...args) => {
    const r = (await cmd("Runtime.evaluate", { expression: `(${f})(...${JSON.stringify(args)})`, awaitPromise: true, returnByValue: true })).result;
    if (r.exceptionDetails) throw new Error(r.exceptionDetails.exception?.description);
    return r.result.value;
  };
  for (let i = 0; i < 50; i++) { if (await js(() => document.querySelectorAll("#games button").length >= 3)) break; await new Promise((r) => setTimeout(r, 200)); }
  const shot = async (name) => writeFileSync(join(shots, name + ".png"), Buffer.from((await cmd("Page.captureScreenshot")).result.data, "base64"));
  return { js, cmd, shot, errors, close: () => { ws.close(); return fetch(`http://127.0.0.1:${port}/json/close/${t.id}`); } };
}
const setup = (page, game, seats) => page.js((game, seats) => {
  localStorage.setItem("npe-arena-v1", JSON.stringify({ last: game, [game]: seats }));
  document.querySelector(`#games [data-id=${game}]`).click();
}, game, seats);
const play = (page, ms = 150000) => page.js(async (ms) => {
  document.querySelector("#run").click();
  const end = Date.now() + ms;
  while (document.querySelector("#run").disabled && Date.now() < end) await new Promise((r) => setTimeout(r, 100));
  const t = (s) => document.querySelector(s).textContent;
  return { status: t("#status"), result: t("#result"), log: t("#log"), turn: t("#turn"), running: document.querySelector("#run").disabled };
}, ms);
const data = JSON.parse(readFileSync(join(root, "arena/data.json")));
const bot = (game, lang) => ({ lang, ...(({ name, code }) => ({ name, code }))(data.games.find((g) => g.toml.startsWith(`id = "${game}"`)).bots.find((b) => b.lang === lang)) });
const js = (code) => ({ lang: "js", name: "evil", code });
const py = (code) => ({ lang: "python", name: "evil.py", code });

const tests = [];
const test = (name, f) => tests.push([name, f]);
const ok = (v, why) => { if (!v) throw new Error(typeof why === "string" ? why : JSON.stringify(why)); };

// ---------- features ----------
for (const game of ["reversi", "snake", "maze"])
  for (const lang of ["js", "python", "c", "cpp"])
    test(`play ${game} ${lang}`, async (p) => {
      await setup(p, game, [bot(game, lang), bot(game, "js")]);
      const r = await play(p);
      ok(r.status.startsWith("对战完成") && !r.log.includes("出局") && r.result.includes("第 1 名"), r);
      await p.shot(`${game}-${lang}`);
    });
test("play rust wasm upload", async (p) => {
  const f = fileURLToPath(new URL("../target/rust-bot/bot.wasm", import.meta.url));
  ok(existsSync(f), "build target/rust-bot/bot.wasm first");
  await setup(p, "snake", [bot("snake", "js"), { lang: "wasm", name: "w", code: "" }]);
  await p.js((b64) => {
    document.querySelectorAll(".pick")[1].click();
    const d = new DataTransfer();
    d.items.add(new File([Uint8Array.from(atob(b64), (c) => c.charCodeAt(0))], "roomy.wasm"));
    const i = document.querySelector("#uploadFile");
    i.files = d.files;
    i.dispatchEvent(new Event("change"));
    return new Promise((r) => setTimeout(r, 300));
  }, readFileSync(f).toString("base64"));
  const r = await play(p);
  ok(r.status.startsWith("对战完成") && !r.log.includes("出局") && r.result.includes("roomy.wasm"), r);
});
test("human plays reversi by clicking", async (p) => {
  await setup(p, "reversi", [{ lang: "human", name: "me", code: "" }, bot("reversi", "js")]);
  const r = await p.js(async () => {
    document.querySelector("#run").click();
    const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
    for (let i = 0; i < 100 && !document.querySelector("#status").textContent.includes("轮到你"); i++) await sleep(100);
    const c = document.querySelector("#board"), b = c.getBoundingClientRect();
    c.dispatchEvent(new MouseEvent("click", { clientX: b.left + b.width * 2.5 / 8, clientY: b.top + b.height * 3.5 / 8 }));
    await sleep(1500);
    const turn = document.querySelector("#turn").textContent;
    document.querySelector("#stop").click();
    await sleep(500);
    return { turn, status: document.querySelector("#status").textContent, run: document.querySelector("#run").disabled };
  });
  ok(r.turn === "2" && r.status === "已停止。" && !r.run, r);
});
test("human plays snake with keys", async (p) => {
  await setup(p, "snake", [{ lang: "human", name: "me", code: "" }]);
  const run = play(p, 60000);
  await new Promise((r) => setTimeout(r, 900));
  await p.js(() => { document.activeElement.blur(); dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowRight" })); });
  const r = await run;
  ok(r.status.startsWith("对战完成") && Number(r.turn.split("/")[1]) > 5 && r.log.includes("没有日志") && r.result.includes("第 1 名"), r);
});
for (const game of ["reversi", "snake", "maze"])
  test(`editor templates ${game}`, async (p) => {
    for (const lang of ["js", "python", "c", "cpp"]) {
      const r = await p.js(async (game, lang) => {
        const { template } = await import("./help.js");
        return template(game, lang);
      }, game, lang);
      await setup(p, game, [{ lang, name: "tpl", code: r }, bot(game, "js")]);
      const out = await play(p);
      ok(out.status.startsWith("对战完成") && !/崩溃|超时|非法/.test(out.log), { lang, ...out });
    }
  });
test("editor marks a C error line", async (p) => {
  await setup(p, "maze", [{ lang: "c", name: "bad.c", code: '#include "maze.h"\nint think(const State *s) {\n    return s->nope;\n}\n' }, bot("maze", "js")]);
  const r = await play(p);
  const marks = await p.js(() => document.querySelector("#editor").shadowRoot.querySelectorAll(".cm-lint-marker-error").length);
  ok(r.log.includes("nope") && marks === 1, { marks, ...r });
  await p.shot("editor-error");
});
for (const [lang, code] of [["js", "let a = 1;\nnpe.run((o) => {\n  return nope.x;\n});"], ["python", "import npe\ndef think(o):\n    return nope\nnpe.run(think)\n"]])
  test(`editor marks a ${lang} runtime error line`, async (p) => {
    await setup(p, "maze", [bot("maze", "js"), { lang, name: "bad", code }]);
    const r = await play(p);
    const m = await p.js(() => {
      const root = document.querySelector("#editor").shadowRoot;
      const mark = root.querySelector(".cm-lint-marker-error")?.getBoundingClientRect().top;
      return [...root.querySelectorAll(".cm-line")].findIndex((l) => Math.abs(l.getBoundingClientRect().top - mark) < 12) + 1;
    });
    ok(m === 3 && r.log.includes("提示"), { m, ...r });
  });
test("editor try, state, docs and completion", async (p) => {
  await setup(p, "reversi", [bot("reversi", "python"), bot("reversi", "js")]);
  const r = await p.js(async () => {
    const t = (s) => document.querySelector(s).textContent;
    document.querySelector("#try").click();
    const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
    while (document.querySelector("#run").disabled) await sleep(100);
    const tried = t("#log");
    document.querySelector("#run").click();
    while (document.querySelector("#run").disabled) await sleep(100);
    document.querySelector("#play").click();
    document.querySelector("[data-tab=state]").click();
    const seek = document.querySelector("#seek");
    seek.value = 2; seek.oninput();
    const state = t("#state");
    document.querySelector("[data-tab=doc]").click();
    document.querySelector("#doc button").click();
    const code = JSON.parse(localStorage.getItem("npe-arena-v1")).reversi[0].code;
    return { tried, state, doc: t("#doc"), code };
  });
  ok(/返回:\{"x":\d,"y":\d\}/.test(r.tried) && r.state.includes("第 2 回合") && r.state.includes('"board"') && r.doc.includes('obs["board"]') && r.code.startsWith('obs["board"]'), r);
  await p.cmd("Input.insertText", { text: "\nobs" });
  await new Promise((r) => setTimeout(r, 800));
  await p.cmd("Input.dispatchKeyEvent", { type: "keyDown", key: "[", text: "[" });
  await new Promise((r) => setTimeout(r, 800));
  await p.shot("editor");
  const list = await p.js(() => [...document.querySelector("#editor").shadowRoot.querySelectorAll(".cm-completionLabel")].map((e) => e.textContent));
  ok(list.includes('obs["moves"]') && !list.includes("0"), list);
  await p.cmd("Input.dispatchKeyEvent", { type: "keyDown", key: "Enter", code: "Enter", windowsVirtualKeyCode: 13 });
  await new Promise((r) => setTimeout(r, 300));
  const line = await p.js(() => JSON.parse(localStorage.getItem("npe-arena-v1")).reversi[0].code.split("\n")[1]);
  ok(line.startsWith('obs["board"]#'), line);
});
test("editor series swaps seats", async (p) => {
  await setup(p, "reversi", [bot("reversi", "js"), bot("reversi", "js")]);
  const r = await p.js(async () => {
    document.querySelector("#series").click();
    while (document.querySelector("#run").disabled) await new Promise((r) => setTimeout(r, 100));
    return document.querySelector("#status").textContent + document.querySelector("#result").textContent;
  });
  ok(r.startsWith("连测完成") && r.includes("已完成 6 / 6 局"), r);
  // Identical deterministic bots: each one sits first three times.
  ok((r.match(/第一名 3 次/g) ?? []).length === 2, r);
});
test("compile error is shown", async (p) => {
  await setup(p, "reversi", [{ lang: "c", name: "bad.c", code: "int think( { }" }, bot("reversi", "js")]);
  const r = await play(p);
  ok(r.status.startsWith("出错了") && r.log.includes("error"), r);
});
test("phone layout has no sideways scroll", async (p) => {
  await p.cmd("Emulation.setDeviceMetricsOverride", { width: 375, height: 812, deviceScaleFactor: 2, mobile: true });
  await new Promise((r) => setTimeout(r, 500));
  await p.shot("phone");
  ok(await p.js(() => document.documentElement.scrollWidth <= 375), "scrolls sideways");
});
test("dark theme", async (p) => {
  await p.js(() => { document.documentElement.dataset.theme = "dark"; });
  await setup(p, "maze", [bot("maze", "js"), bot("maze", "js")]);
  ok((await play(p)).status.startsWith("对战完成") && Number(await p.js(() => document.querySelector("#seek").max)) > 5, "did not finish");
  await p.shot("dark");
});

// ---------- attacks ----------
const survive = (r) => ok(!r.running && r.status.startsWith("对战完成") && !r.log.includes("worker error") && !/(runner|space|light)\.(js|py) (崩溃|超时|非法)/.test(r.log) && Number(r.turn.split("/")[1]) > 5, r);
const PROBE = `
const out = [];
const probe = async (name, f) => { try { const v = await f(); out.push(name + "=REACHED " + v); } catch (e) { out.push(name + "=blocked"); } };
`;
test("attack js reads the site", async (p) => {
  await p.js(() => { localStorage.setItem("secret", "s3cr3t"); document.cookie = "sid=s3cr3t"; });
  hits.length = 0;
  const base = new URL(BASE).origin;
  await setup(p, "maze", [js(`${PROBE}
    const all = (async () => {
      await probe("window", () => window.document.title);
      await probe("storage", () => localStorage.getItem("secret"));
      await probe("cookie", () => document.cookie);
      await probe("idb", () => new Promise((a, b) => { const r = indexedDB.open("x"); r.onsuccess = () => a("open"); r.onerror = b; }));
      await probe("caches", () => caches.open("x"));
      await probe("fetch-site", () => fetch("${base}/STOLEN-fetch"));
      await probe("fetch-nocors", () => fetch("${base}/STOLEN-nocors?d=1", { mode: "no-cors" }));
      await probe("fetch-ext", () => fetch("https://cdn.jsdelivr.net/npm/pyodide@314.0.8/package.json"));
      await probe("xhr", () => new Promise((a, b) => { const x = new XMLHttpRequest(); x.onload = () => a("ok"); x.onerror = b; x.open("GET", "${base}/STOLEN-xhr"); x.send(); }));
      await probe("ws", () => new Promise((a, b) => { const w = new WebSocket("${base.replace("http", "ws")}/STOLEN-ws"); w.onopen = () => a("ok"); w.onerror = b; }));
      await probe("sse", () => new Promise((a, b) => { const w = new EventSource("${base}/STOLEN-sse"); w.onopen = () => a("ok"); w.onerror = b; }));
      await probe("import", () => import("${base}/arena/app.js"));
      await probe("import-data", () => import("data:text/javascript,export default 1"));
      await probe("importScripts", () => importScripts("${base}/STOLEN-is.js"));
      await probe("beacon", () => { if (!navigator.sendBeacon) throw 0; return navigator.sendBeacon("${base}/STOLEN-beacon", "x"); });
      await probe("rtc", () => new RTCPeerConnection());
      await probe("worker", () => new Promise((a, b) => { const w = new Worker("${base}/arena/app.js"); w.onerror = b; w.onmessage = a; setTimeout(b, 1000); }));
      await probe("blob-fetch", () => fetch(URL.createObjectURL(new Blob(["x"]))));
      console.log("PROBE " + out.join(";"));
    })();
    npe.run((o) => ({ dir: 4 }));`), bot("maze", "python")]);
  const r = await play(p);
  survive(r);
  const line = /PROBE (.*)/.exec(r.log)?.[1] ?? "";
  ok(line.split(";").length === 18 && !line.includes("REACHED"), line || r);
  ok(!hits.some((h) => h.includes("STOLEN")), hits.filter((h) => h.includes("STOLEN")));
});
test("attack python escapes through js module", async (p) => {
  hits.length = 0;
  const base = new URL(BASE).origin;
  await setup(p, "maze", [py(`
import js, npe
def probe(name, f):
    try:
        f(); print("P", name, "REACHED")
    except BaseException as e:
        print("P", name, "blocked")
def xhr(u):
    x = js.XMLHttpRequest.new(); x.open("GET", u, False); x.send()
probe("window", lambda: js.window.document)
probe("storage", lambda: js.localStorage.getItem("secret"))
probe("xhr", lambda: xhr("${base}/STOLEN-py"))
probe("xhr-ext", lambda: xhr("https://example.com/"))
probe("xhr-cdn-other", lambda: xhr("https://cdn.jsdelivr.net/npm/lodash@4.17.21/package.json"))
js.fetch("${base}/STOLEN-pyfetch")
npe.run(lambda o: {"dir": 4})
`), bot("maze", "js")]);
  const r = await play(p);
  survive(r);
  ok((r.log.match(/P \w[\w-]* blocked/g) ?? []).length === 5 && !r.log.includes("REACHED"), r);
  ok(!hits.some((h) => h.includes("STOLEN")), hits);
});
test("attack spoofed and malformed replies", async (p) => {
  await setup(p, "snake", [js(`
    npe.run((o) => {
      const junk = ["{", "null", '{"turn":' + o.turn + ',"action":{"dir":99}}', '{"__proto__":{"x":1}}', "x".repeat(70000)];
      for (const t of junk) postMessage({ kind: "reply", text: t });
      postMessage({ kind: "built", wasm: new Uint8Array(4) });
      postMessage({ kind: "hello" });
      postMessage(null); postMessage(5); postMessage({ kind: "reply", text: { a: 1 } });
      return { dir: 1 };
    });`), bot("snake", "js")]);
  const r = await play(p);
  survive(r);
  ok(await p.js(() => ({}).x === undefined), "prototype polluted");
});
test("attack exponential log value", async (p) => {
  await setup(p, "maze", [js(`let a = ["x"]; for (let i = 0; i < 40; i++) a = [a, a];
    npe.run((o) => { for (const kind of ["log", "status"]) postMessage({ kind, text: a }); return { dir: 4 }; });`), bot("maze", "js")]);
  survive(await play(p, 60000));
});
test("attack deeply nested action", async (p) => {
  await setup(p, "maze", [js(`npe.run((o) => { let a = 1; for (let i = 0; i < 3000; i++) a = [a]; return a; });`), bot("maze", "js")]);
  const r = await play(p, 60000);
  survive(r);
  ok(r.log.includes("非法操作"), r);
});
test("stop during loading", async (p) => {
  await setup(p, "maze", [bot("maze", "python"), js("npe.run((o) => { for (;;); })")]);
  const ms = await p.js(async () => {
    const t = performance.now();
    document.querySelector("#run").click();
    await new Promise((r) => setTimeout(r, 400));
    document.querySelector("#stop").click();
    while (document.querySelector("#run").disabled) await new Promise((r) => setTimeout(r, 20));
    return performance.now() - t;
  });
  ok(ms < 2000 && (await p.js(() => document.querySelectorAll("iframe").length)) === 0, String(ms));
});
test("series keeps the editor on its own seat", async (p) => {
  await setup(p, "reversi", [js("/*A*/" + bot("reversi", "js").code), js("/*B*/" + bot("reversi", "js").code)]);
  const r = await p.js(async () => {
    const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
    document.querySelector("#series").click();
    while (!document.querySelector("#result").textContent.includes("已完成 1")) await sleep(5);
    document.querySelectorAll(".pick")[1].click();
    while (document.querySelector("#run").disabled) await sleep(20);
    return document.querySelector("#editor").shadowRoot.querySelector(".cm-line").textContent;
  });
  ok(r.includes("/*B*/"), r);
});
test("attack message flood", async (p) => {
  await setup(p, "maze", [js(`npe.run((o) => { for (;;) postMessage({ kind: "log", text: "x".repeat(1000) }); });`), bot("maze", "js")]);
  const r = await play(p, 90000);
  survive({ ...r, log: r.log.replace(/x{20,}/g, "x…") });
  ok(r.log.includes("too many messages"), "flood was not stopped");
});
test("attack giant messages", async (p) => {
  await setup(p, "maze", [js(`const big = "x".repeat(2e7); npe.run((o) => { for (;;) postMessage({ kind: "log", text: big }); });`), bot("maze", "js")]);
  const r = await play(p, 90000);
  survive({ ...r, log: r.log.slice(0, 500) });
});
test("attack endless loop at load and in think", async (p) => {
  await setup(p, "maze", [js(`npe.run((o) => { if (o.turn > 2) for (;;); return { dir: 4 }; });`), bot("maze", "js")]);
  const r = await play(p, 60000);
  survive(r);
  ok(r.log.includes("超时"), r);
});
test("attack c memory error kills only that bot", async (p) => {
  await setup(p, "maze", [{ lang: "c", name: "segv.c", code: '#include "maze.h"\nint think(const State *s) { printf("hi %d\\n", s->turn); if (s->turn == 5) { volatile char *p = (char *)0xfffffff0u; *p = 1; } return STAY; }' }, bot("maze", "js")]);
  const r = await play(p);
  survive(r);
  ok(r.log.includes("memory error") && r.log.includes("hi 4"), r);
});
test("attack markup in names, logs and replays", async (p) => {
  const x = `<img src=x onerror="window.PWNED=1"><script>window.PWNED=1</script>`;
  await setup(p, "maze", [{ ...js(`npe.run((o) => { console.log(${JSON.stringify(x)}); return { dir: 4 }; });`), name: x }, bot("maze", "js")]);
  survive(await play(p));
  const load = (r) => p.js((text) => {
    const d = new DataTransfer();
    d.items.add(new File([text], "r.json"));
    const i = document.querySelector("#importFile");
    i.files = d.files;
    i.dispatchEvent(new Event("change"));
    return new Promise((r) => setTimeout(() => r(document.querySelector("#status").textContent), 400));
  }, typeof r === "string" ? r : JSON.stringify(r));
  const base = {
    version: 1, game: x, title: x, seed: 1, players: [{ name: x, lang: x }], logs: [{ turn: 0, player: 0, text: x }],
    render: { kind: "grid", width: 5, height: 5, sprites: { a: { shape: x, color: `red;background:url(//evil)`, size: 1e9 }, ring: { shape: "ring", size: 0.05 } }, fps: 1e9 },
    frames: [{ turn: 0, view: { entities: [{ type: "a", x: 1e300, y: "q", player: -5 }, { type: "ring", x: 1, y: 1 }, { type: "__proto__" }, { type: "constructor" }], players: [{ note: x, score: x }] }, events: [{ player: 99, kind: "__proto__", detail: x }] }],
    outcome: { ranks: [x], scores: [null], reason: x },
  };
  ok((await load(base)).startsWith("已打开"), "valid hostile replay should open");
  for (const bad of ["{", "[]", "null", "5", { ...base, version: 2 }, { ...base, render: { ...base.render, width: 1e9 } }, { ...base, frames: Array(6000).fill(base.frames[0]) }, { ...base, frames: [null] }, { ...base, frames: [{ view: { entities: [null] } }] }, { ...base, players: "x" }])
    ok((await load(bad)).startsWith("打不开"), { accepted: String(JSON.stringify(bad)).slice(0, 80) });
  ok(await p.js(() => window.PWNED === undefined && !document.querySelector("main img, main script")), "markup was executed");
});
test("attack sandbox page opened or framed without sandbox", async (p) => {
  const r = await p.js(async () => {
    const f = document.createElement("iframe");
    f.src = "sandbox/run.html";
    let hello = false;
    addEventListener("message", (e) => { if (e.source === f.contentWindow) hello = true; });
    document.body.append(f);
    await new Promise((r) => setTimeout(r, 1000));
    f.contentWindow.postMessage({ kind: "boot", lang: "js", code: "npe.run(()=>0)" }, "*");
    await new Promise((r) => setTimeout(r, 500));
    return { hello, text: f.contentDocument.body.textContent };
  });
  ok(!r.hello && r.text.includes("only works inside"), r);
});
test("attack inline script and eval in the main page", async (p) => {
  const r = await p.js(() => {
    const s = document.createElement("script");
    s.textContent = "window.INLINE=1";
    document.body.append(s);
    document.body.insertAdjacentHTML("beforeend", `<img src="data:," onerror="window.INLINE=1">`);
    return new Promise((r) => setTimeout(() => r(window.INLINE === undefined), 400));
  });
  ok(r, "inline script ran");
});
test("feedback link is prefilled and isolated", async (p) => {
  const r = await p.js(() => {
    let call;
    window.open = (...a) => { call = a.map(String); };
    document.querySelector("#feedbackBtn").click();
    document.querySelector("#fbText").value = "a&b=c#d <b>";
    document.querySelector("#fbOpen").click();
    return call;
  });
  const u = new URL(r[0]);
  ok(u.host === "github.com" && u.pathname.endsWith("/issues/new") && u.searchParams.get("body").startsWith("a&b=c#d <b>") && r[2].includes("noopener"), r);
  await p.shot("feedback");
});

let failed = 0;
const want = process.argv[2] ?? "";
for (const [name, f] of tests.filter((t) => t[0].includes(want))) {
  const t0 = Date.now();
  const p = await open(BASE);
  try {
    await f(p);
    if (p.errors.length) throw new Error("page error: " + p.errors.join(" | ").slice(0, 500));
    console.log("ok  ", name, `${((Date.now() - t0) / 1000).toFixed(1)}s`);
  } catch (e) {
    failed++;
    console.log("FAIL", name, e.message.slice(0, 1200));
  }
  await p.close();
}
chrome.kill();
server.close();
await new Promise((r) => setTimeout(r, 800));
try { rmSync(profile, { recursive: true, force: true }); } catch {}
console.log(failed ? `${failed} failed` : "all passed");
process.exit(failed ? 1 : 0);
