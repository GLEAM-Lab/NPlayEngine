// JavaScript and WASI bots. No network: the frame's CSP has no connect-src.
const post = self.postMessage.bind(self);
const log = (text) => post({ kind: "log", text });
const show = (v) => { try { return typeof v === "string" ? v : JSON.stringify(v); } catch { return String(v); } };
for (const k of ["log", "info", "warn", "error", "debug"]) console[k] = (...a) => log(a.map(show).join(" "));
let handle = null;

function bootJs(code) {
  let think = null;
  const npe = { run: (f) => { think = f; } };
  new Function("npe", code)(npe);
  if (typeof think !== "function") throw new Error("the bot never called npe.run(think)");
  handle = (line) => {
    const m = JSON.parse(line);
    if (m.type === "init") return '{"ready":true}';
    if (m.type === "turn") return JSON.stringify({ turn: m.turn, action: think(m.obs) });
    return "";
  };
}

async function bootWasm(bytes) {
  const module = await WebAssembly.compile(bytes);
  let x;
  const view = () => new DataView(x.memory.buffer);
  const dec = new TextDecoder();
  let rest = "", seed = 1;
  const write = (fd, iov, n, out) => {
    const v = view();
    let total = 0;
    for (let i = 0; i < n; i++) {
      const p = v.getUint32(iov + 8 * i, true), len = v.getUint32(iov + 8 * i + 4, true);
      if (rest.length < 8192) rest += dec.decode(new Uint8Array(x.memory.buffer, p, Math.min(len, 4096)));
      total += len;
    }
    const lines = rest.split("\n");
    rest = lines.pop();
    lines.forEach(log);
    v.setUint32(out, total, true);
    return 0;
  };
  // The bot gets no files, no arguments and no environment.
  const zero2 = (a, b) => { view().setUint32(a, 0, true); view().setUint32(b, 0, true); return 0; };
  const wasi = {
    fd_write: write,
    fd_read: (fd, iov, n, out) => { view().setUint32(out, 0, true); return 0; },
    fd_close: () => 0,
    fd_seek: () => 70,
    fd_fdstat_get: (fd, p) => { for (let i = 0; i < 24; i++) view().setUint8(p + i, 0); view().setUint8(p, 2); return fd < 3 ? 0 : 8; },
    fd_prestat_get: () => 8,
    environ_sizes_get: zero2, environ_get: () => 0,
    args_sizes_get: zero2, args_get: () => 0,
    clock_time_get: (id, prec, p) => { view().setBigUint64(p, BigInt(Math.floor(performance.now() * 1e6)), true); return 0; },
    clock_res_get: (id, p) => { view().setBigUint64(p, 1000000n, true); return 0; },
    random_get: (p, len) => { for (let i = 0; i < len; i++) { seed = (seed * 1103515245 + 12345) >>> 0; view().setUint8(p + i, seed >>> 16); } return 0; },
    sched_yield: () => 0,
    proc_exit: (c) => { throw new Error("the bot exited with code " + c); },
  };
  const imports = {};
  for (const i of WebAssembly.Module.imports(module)) {
    if (i.kind !== "function") throw new Error("unsupported import " + i.module + "." + i.name);
    (imports[i.module] ??= {})[i.name] = (i.module === "wasi_snapshot_preview1" && wasi[i.name]) || (() => 52);
  }
  x = (await WebAssembly.instantiate(module, imports)).exports;
  if (!x.npe_buf || !x.npe_line || !x.memory) throw new Error("not an NPlayEngine bot: npe_buf and npe_line are missing");
  x._initialize?.();
  handle = (line) => {
    const b = new TextEncoder().encode(line);
    const p = x.npe_buf(b.length);
    new Uint8Array(x.memory.buffer, p, b.length).set(b);
    const r = x.npe_line();
    const mem = new Uint8Array(x.memory.buffer);
    let e = r;
    while (e < mem.length && mem[e] && e - r < 65536) e++;
    return dec.decode(mem.slice(r, e));
  };
}

const explain = (e) => {
  const t = String((e && e.stack) || e);
  return t.includes("unreachable") || t.includes("out of bounds") ? "memory error or abort: " + t : t;
};

self.addEventListener("message", async ({ data: m }) => {
  try {
    if (m.kind === "boot") {
      if (m.lang === "js") bootJs(m.code);
      else await bootWasm(m.wasm);
      post({ kind: "booted" });
    } else if (m.kind === "line") {
      const r = handle(m.text);
      if (r) post({ kind: "reply", text: r });
    }
  } catch (e) {
    post({ kind: "crash", text: explain(e) });
  }
});
