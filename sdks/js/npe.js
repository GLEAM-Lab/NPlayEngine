// NPlayEngine JavaScript SDK. Usage: node npe.js bot.js
// The bot calls npe.run(think), where think(obs) returns a plain object.
const fs = require("fs");
const { createInterface } = require("readline");
const { Console } = require("console");
globalThis.console = new Console(process.stderr, process.stderr);
let think;
globalThis.npe = { run: (f) => { think = f; } };
new Function("npe", fs.readFileSync(process.argv[2], "utf8"))(globalThis.npe);
createInterface({ input: process.stdin }).on("line", (line) => {
  const msg = JSON.parse(line);
  if (msg.type === "init") process.stdout.write('{"ready":true}\n');
  if (msg.type === "turn") process.stdout.write(JSON.stringify({ turn: msg.turn, action: think(msg.obs) }) + "\n");
});
