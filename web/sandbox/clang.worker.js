// Compiles C and C++ with the pinned YoWASP Clang build. Student code is only
// compiled here, never run. The result runs in run.html, which has no network.
const post = self.postMessage.bind(self);
let clang = null;

self.addEventListener("message", async ({ data: m }) => {
  try {
    if (m.kind === "boot") {
      post({ kind: "status", text: "正在加载 C/C++ 编译器,首次约 30 MB" });
      clang = await import("@CLANG@gen/bundle.js");
      post({ kind: "booted" });
    } else if (m.kind === "build") {
      const cpp = m.lang === "cpp";
      const name = cpp ? "bot.cpp" : "bot.c";
      const args = [
        cpp ? "-std=c++17" : "-std=c11", "-O2", "-mexec-model=reactor", "-I.", "-Wl,--stack-first", "-Wl,-z,stack-size=1048576",
        ...(cpp ? ["-fno-exceptions"] : []), name, "-o", "bot.wasm",
      ];
      let log = "";
      const sink = (b) => { if (b && log.length < 8000) log += new TextDecoder().decode(b); };
      const tool = clang.commands[cpp ? "clang++" : "clang"];
      try {
        const out = await tool(args, { ...m.headers, [name]: m.code }, { stdout: sink, stderr: sink });
        post({ kind: "built", wasm: new Uint8Array(out["bot.wasm"]) });
      } catch (e) {
        post({ kind: "built", error: log || String(e) });
      }
    }
  } catch (e) {
    post({ kind: "crash", text: String((e && e.message) || e) });
  }
});
