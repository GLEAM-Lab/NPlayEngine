// Replays the scripted matches from crates/npe-core/tests/games.rs on the wasm
// build and compares them byte for byte with the native golden files.
import { readFileSync } from "node:fs";
const root = new URL("..", import.meta.url);
const read = (p) => readFileSync(new URL(p, root));
const { instance } = await WebAssembly.instantiate(read("target/wasm32-unknown-unknown/release/npe_wasm.wasm"));
const x = instance.exports;
function call(req) {
  const b = new TextEncoder().encode(JSON.stringify(req));
  const p = x.npe_alloc(b.length);
  new Uint8Array(x.memory.buffer, p, b.length).set(b);
  const r = BigInt.asUintN(64, x.npe_call(p, b.length));
  x.npe_free(p, b.length);
  const ptr = Number(r >> 32n), len = Number(r & 0xffffffffn);
  const text = new TextDecoder().decode(new Uint8Array(x.memory.buffer, ptr, len));
  x.npe_free(ptr, len);
  const v = JSON.parse(text);
  if (v.err) throw new Error(v.err);
  return { v: v.ok, text };
}
let bad = 0;
for (const [id, n] of [["reversi", 2], ["snake", 6], ["maze", 4]]) {
  const players = Array.from({ length: n }, (_, i) => ({ name: `P${i}`, lang: "test" }));
  const t0 = performance.now();
  let s = call({ op: "new", toml: `${read(`games/${id}/game.toml`)}`, script: `${read(`games/${id}/rules.rhai`)}`, seed: 42, players }).v;
  while (!s.outcome) {
    const replies = {};
    for (const p of s.active) {
      const mv = s.messages[p].obs.moves;
      const action = id === "reversi" ? mv[s.turn % mv.length] : { dir: (Math.floor(s.turn / 4) + p) % 4 };
      replies[p] = { kind: "action", action };
    }
    s = call({ op: "step", id: s.id, replies }).v;
  }
  const got = call({ op: "replay", id: s.id }).text.replace(/^\{"ok":/, "").replace(/\}$/, "");
  // The wasm path goes through serde_json::Value, which sorts keys. Compare values.
  const sort = (v) => Array.isArray(v) ? v.map(sort) : v && typeof v === "object" ? Object.fromEntries(Object.keys(v).sort().map((k) => [k, sort(v[k])])) : v;
  const same = JSON.stringify(sort(JSON.parse(got))) === JSON.stringify(sort(JSON.parse(read(`target/golden/${id}.json`))));
  console.log(id, same ? "identical" : "DIFFERENT", `turns=${s.turn}`, `${(performance.now() - t0).toFixed(0)} ms`);
  if (!same) bad = 1;
}
process.exit(bad);
