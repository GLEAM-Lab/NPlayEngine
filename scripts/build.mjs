// Builds the static site into dist/arena. Copy that directory to the homepage
// repository as public/teach/arena.
import { execFileSync } from "node:child_process";
import { cpSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { homedir } from "node:os";
import { extname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = fileURLToPath(new URL("..", import.meta.url));
const out = join(root, "dist", "arena");
const text = (...p) => readFileSync(join(root, ...p), "utf8").replaceAll("\r\n", "\n");

// npm is a .cmd file on Windows, so call esbuild through node.
execFileSync(process.execPath, ["node_modules/esbuild/bin/esbuild", "editor.src.js", "--bundle", "--format=esm", "--minify", "--target=es2022", "--legal-comments=eof", "--outfile=../editor.js", "--log-level=warning"], { cwd: join(root, "web/editor"), stdio: "inherit" });
execFileSync("cargo", ["build", "--release", "-p", "npe-wasm", "--target", "wasm32-unknown-unknown"], {
  cwd: root, stdio: "inherit",
  // Keep local paths out of the published file.
  env: { ...process.env, CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_RUSTFLAGS: `--remap-path-prefix=${process.env.CARGO_HOME ?? join(homedir(), ".cargo")}=cargo --remap-path-prefix=${root}=src/` },
});
rmSync(out, { recursive: true, force: true });
mkdirSync(join(out, "sandbox"), { recursive: true });
for (const f of ["index.html", "app.js", "engine.js", "viewer.js", "editor.js", "help.js", "theme.js", "style.css", "config.json"]) cpSync(join(root, "web", f), join(out, f));
cpSync(join(root, "target/wasm32-unknown-unknown/release/npe_wasm.wasm"), join(out, "npe.wasm"));

// Versions are pinned. Each sandbox page may reach its own directory and nothing else.
const PYODIDE = "https://cdn.jsdelivr.net/npm/pyodide@314.0.8/";
const CLANG = "https://cdn.jsdelivr.net/npm/@yowasp/clang@22.0.0-git20695-13/";
const csp = (cdn) =>
  `default-src 'none'; script-src 'unsafe-inline' 'unsafe-eval' blob:${cdn ? " " + cdn : ""}; worker-src blob:; connect-src ${cdn || "'none'"}; base-uri 'none'; form-action 'none'`;
const frame = text("web/sandbox/frame.html");
for (const [name, cdn] of [["run", ""], ["python", PYODIDE], ["clang", CLANG]]) {
  const worker = text(`web/sandbox/${name}.worker.js`).replaceAll("@PYODIDE@", PYODIDE).replaceAll("@CLANG@", CLANG);
  if (/<\/script/i.test(worker)) throw new Error("worker source must not contain a closing script tag");
  const html = frame.replace("@CSP@", () => csp(cdn)).replace("@CAN_BUILD@", () => String(name === "clang")).replace("@WORKER@", () => worker);
  writeFileSync(join(out, "sandbox", `${name}.html`), html);
}

const LANG = { ".py": "python", ".js": "js", ".c": "c", ".cpp": "cpp", ".java": "java", ".rs": "rust" };
const games = ["reversi", "snake", "maze"].map((id) => ({
  toml: text("games", id, "game.toml"),
  rules: text("games", id, "rules.rhai"),
  bots: readdirSync(join(root, "bots", id)).map((f) => ({ name: f, lang: LANG[extname(f)], code: text("bots", id, f) })),
}));
const headers = Object.fromEntries(readdirSync(join(root, "sdks/c")).map((f) => [f, text("sdks/c", f)]));
const version = /\nversion = "(.+?)"/.exec(text("Cargo.toml"))[1];
writeFileSync(join(out, "data.json"), JSON.stringify({ version, games, headers }));

// License texts of the code bundled into editor.js.
const lock = JSON.parse(text("web/editor/package-lock.json")).packages;
const notices = Object.keys(lock).filter((k) => k && !lock[k].dev && !lock[k].optional).map((k) => {
  const file = readdirSync(join(root, "web/editor", k)).find((f) => /^licen[sc]e/i.test(f));
  return `${k.replace("node_modules/", "")} ${lock[k].version} (${lock[k].license})\n\n${file ? text("web/editor", k, file) : ""}`;
});
writeFileSync(join(out, "THIRD_PARTY.txt"), notices.join("\n\n----------\n\n"));
console.log("built", out);
