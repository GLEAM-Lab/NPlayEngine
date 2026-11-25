// Python bots. The CSP allows the pinned Pyodide directory and nothing else.
const post = self.postMessage.bind(self);
let handle = null;

self.addEventListener("message", async ({ data: m }) => {
  try {
    if (m.kind === "boot") {
      post({ kind: "status", text: "正在加载 Python" });
      const { loadPyodide } = await import("@PYODIDE@pyodide.mjs");
      const out = (text) => post({ kind: "log", text });
      const py = await loadPyodide({ indexURL: "@PYODIDE@", stdout: out, stderr: out });
      py.runPython(`
import sys, types, json
_npe = types.ModuleType("npe")
def _run(think): _npe.think = think
_npe.run = _run
_npe.think = None
sys.modules["npe"] = _npe
def _npe_line(line):
    m = json.loads(line)
    if m["type"] == "init":
        return '{"ready":true}'
    if m["type"] == "turn":
        return json.dumps({"turn": m["turn"], "action": _npe.think(m["obs"])})
    return ""
`);
      handle = py.globals.get("_npe_line");
      const scope = py.toPy({ __name__: "__main__" });
      py.runPython(m.code, { globals: scope, filename: "bot.py" });
      if (!py.runPython("_npe.think is not None")) throw new Error("the bot never called npe.run(think)");
      post({ kind: "booted" });
    } else if (m.kind === "line") {
      const r = handle(m.text);
      if (r) post({ kind: "reply", text: r });
    }
  } catch (e) {
    // The last lines of a Python traceback say what happened.
    const t = String((e && e.message) || e).trim().split("\n");
    post({ kind: "crash", text: t.slice(-6).join("\n") });
  }
});
