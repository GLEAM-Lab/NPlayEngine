// Checks the local studio: every language template runs, and foreign pages get nothing.
import { spawn } from "node:child_process";
import { request } from "node:http";
import { fileURLToPath } from "node:url";
import { template } from "../web/help.js";
const PORT = 8799, base = `http://127.0.0.1:${PORT}`;
const exe = fileURLToPath(new URL(`../target/release/npe${process.platform === "win32" ? ".exe" : ""}`, import.meta.url));
const child = spawn(exe, ["studio", "--port", String(PORT)], { stdio: "ignore" });
await new Promise((r) => setTimeout(r, 1500));
const call = (path, { method = "GET", headers = {}, body } = {}) => new Promise((res, rej) => {
  const q = request({ host: "127.0.0.1", port: PORT, path, method, headers: { Host: `127.0.0.1:${PORT}`, ...headers, ...(body ? { "Content-Length": Buffer.byteLength(body) } : {}) } }, (r) => {
    let t = ""; r.on("data", (d) => (t += d)); r.on("end", () => res({ status: r.statusCode, text: t, headers: r.headers }));
  });
  q.on("error", rej);
  q.end(body);
});
let failed = 0;
const check = (name, ok, info) => { if (!ok) failed++; console.log(ok ? "ok  " : "FAIL", name, ok ? "" : JSON.stringify(info).slice(0, 600)); };
try {
  const { token } = JSON.parse((await call("/api/token", { headers: { "Sec-Fetch-Site": "same-origin" } })).text);
  const post = (body, headers = {}) => call("/api/run", { method: "POST", headers: { Origin: base, "X-Npe-Token": token, ...headers }, body: JSON.stringify(body) });
  for (const game of ["reversi", "snake", "maze"]) {
    const langs = ["c", "cpp", "python", "java", "rust", "js"];
    const bots = langs.map((lang) => ({ name: lang, lang, code: template(game, lang) }));
    const max = { reversi: 2, snake: 6, maze: 4 }[game];
    for (let i = 0; i < 6; i += max) {
      const set = bots.slice(i, i + max);
      const r = await post({ game, seed: 1, bots: set });
      const v = r.status === 200 ? JSON.parse(r.text) : null;
      check(`templates ${game} ${set.map((b) => b.lang)}`, v?.outcome && v.frames.every((f) => !f.events?.length), r.text.slice(0, 500));
    }
  }
  const good = { game: "maze", seed: 1, bots: [{ name: "a", lang: "js", code: template("maze", "js") }] };
  check("foreign origin", (await post(good, { Origin: "https://evil.example" })).status === 403);
  check("no origin", (await post(good, { Origin: "" })).status === 403);
  check("null origin", (await post(good, { Origin: "null" })).status === 403);
  check("wrong token", (await post(good, { "X-Npe-Token": "x" })).status === 403);
  check("dns rebinding", (await post(good, { Host: `evil.example:${PORT}`, Origin: `http://evil.example:${PORT}` })).status === 403);
  check("cross-site token read", (await call("/api/token", { headers: { "Sec-Fetch-Site": "cross-site" } })).status === 403);
  check("no CORS header", !(await call("/api/token")).headers["access-control-allow-origin"]);
  for (const p of ["/../Cargo.toml", "/..%2fCargo.toml", "/sandbox/../../Cargo.toml", "/C:/Windows/win.ini"]) check(`traversal ${p}`, (await call(p)).status === 404);
  for (const game of ["../games/maze", "C:/x", "", ".", "maze/../maze"]) check(`game id ${game}`, (await post({ ...good, game })).status === 400);
  for (const code of ["public class ../../x {}", "public class CON {}", "public class Npe {}", "x"]) check(`java name ${code}`, (await post({ ...good, bots: [{ name: "a", lang: "java", code }] })).status === 400);
  check("unknown language", (await post({ ...good, bots: [{ name: "a", lang: "native", code: "x" }] })).status === 400);
  check("too many players", (await post({ ...good, bots: Array(50).fill(good.bots[0]) })).status === 400);
  check("huge body", [413, 400].includes((await post({ ...good, pad: "x".repeat(3e6) }).catch(() => ({ status: 413 }))).status));
} finally {
  child.kill();
}
console.log(failed ? `${failed} failed` : "all passed");
process.exit(failed ? 1 : 0);
