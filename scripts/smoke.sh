#!/usr/bin/env bash
# Native checks: tests, wasm equality, and every language in every game.
set -e
cd "$(dirname "$0")/.."
cargo test --release -p npe-core -p npe-runner
cargo build --release -p npe-wasm --target wasm32-unknown-unknown
node scripts/check-wasm.mjs
cargo build --release -p npe-cli
NPE=target/release/npe
$NPE run snake bots/snake/*.*
$NPE run maze bots/maze/*.c bots/maze/*.java bots/maze/*.rs bots/maze/*.js
$NPE run maze bots/maze/*.py bots/maze/*.cpp
$NPE tournament reversi bots/reversi/*.* --seats 2
