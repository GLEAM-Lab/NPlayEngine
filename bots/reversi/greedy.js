// 黑白棋:选翻子最多的一步。
const DIRS = [[1, 0], [1, 1], [0, 1], [-1, 1], [-1, 0], [-1, -1], [0, -1], [1, -1]];

function flips(obs, x, y) {
  let total = 0;
  for (const [dx, dy] of DIRS) {
    let k = 0, i = x + dx, j = y + dy;
    while (obs.board[j]?.[i] === obs.opponent) { k++; i += dx; j += dy; }
    if (k && obs.board[j]?.[i] === obs.me) total += k;
  }
  return total;
}

function think(obs) {
  let best = obs.moves[0];
  for (const m of obs.moves) if (flips(obs, m.x, m.y) > flips(obs, best.x, best.y)) best = m;
  return best;
}

npe.run(think);
