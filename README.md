# NPlayEngine

学生写 AI 互相对战的平台。内核用 Rust 编写，同一份内核编译成本地命令行和网页 WASM，两边的对局结果一致。

| 游戏 | 人数 | 特点 |
| --- | --- | --- |
| 黑白棋 reversi | 2 | 轮流落子 |
| 贪吃蛇 snake | 1 到 6 | 同时行动 |
| 迷宫拾光 maze | 1 到 4 | 同时行动，视野有限 |

| 语言 | 网页版 | 本地版 |
| --- | --- | --- |
| Python、JavaScript、C、C++ | 在浏览器里编译和运行 | 支持 |
| Rust | 上传编译好的 `.wasm` | 支持源码 |
| Java | 暂不支持 | 支持源码 |

## 使用

```bash
cargo build --release -p npe-cli
npe games
npe run snake bots/snake/bfs.py bots/snake/nearest.c --seed 1 --replay r.json
npe tournament reversi bots/reversi/*.* --seats 2 --rounds 3
npe studio      # 本地网页，支持全部语言
```

本地版会以当前用户的权限运行选手程序，只运行可信的代码。原因见 [docs/SECURITY.md](docs/SECURITY.md)。

## 构建网页

```bash
rustup target add wasm32-unknown-unknown
(cd web/editor && npm ci)
node scripts/build.mjs        # 产物在 dist/arena
```

把 `dist/arena` 复制到主页仓库的 `public/teach/arena`，发布后的地址是 `/teach/arena/`。页面全部使用相对路径，不需要改主页的构建配置。命令行程序内嵌这份产物，所以要先构建网页，再构建 `npe-cli`。

反馈按钮打开哪个仓库的 Issue，由 [web/config.json](web/config.json) 的 `feedbackRepo` 决定。

## Rust 选手编译成 WASM

```bash
rustup target add wasm32-wasip1
rustc --target wasm32-wasip1 --edition=2021 -O --crate-type=rlib --crate-name=npe -o libnpe.rlib sdks/rust/npe.rs
rustc --target wasm32-wasip1 --edition=2021 -O --crate-type=cdylib --crate-name=bot -o bot.wasm my_bot.rs --extern npe=libnpe.rlib
```

## 新增游戏

在 `games/<id>/` 下写两个文件：`game.toml` 声明人数、时限、渲染方式和操作方式，`rules.rhai` 实现 9 个函数：`init`、`active`、`observe`、`validate`、`default_action`、`eliminate`、`apply`、`outcome`、`view`。再把 id 加进 `scripts/build.mjs`，把字段说明加进 `web/help.js`，把 C 头文件放进 `sdks/c/`。

规则脚本用 Rhai，没有用 Luau。`mlua` 只能编译到 Emscripten，Rhai 是纯 Rust，可以直接编译到 `wasm32-unknown-unknown`，产物没有任何导入函数。

## 通信协议

每行一个 JSON。引擎先发 `{"type":"init",...}`，选手回 `{"ready":true}`。每回合引擎发 `{"type":"turn","turn":n,"obs":{...}}`，选手回 `{"turn":n,"action":{...}}`。回合号对不上的回复会被丢弃，所以超时后的迟到回复不会影响下一回合。

浏览器里没有阻塞式的标准输入，所以 C、C++、Rust 的 SDK 在 wasm 目标下导出 `npe_buf` 和 `npe_line`，处理同样的文本行。网页版的单步时限放宽到 2 秒，超时的选手会直接出局，本地版允许少量超时。

## 测试

```bash
bash scripts/smoke.sh         # 内核、恶意选手、WASM 一致性、全部语言
node scripts/e2e.mjs          # 真实浏览器里的功能和攻击测试，Rust 用例需先编译 target/rust-bot/bot.wasm
node scripts/studio-test.mjs  # 本地服务的功能和攻击测试
```
