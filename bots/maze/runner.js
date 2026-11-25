// 迷宫拾光:优先拿护盾,其次拿光点,绕开哨兵。
const DIRS = [[0, -1], [1, 0], [0, 1], [-1, 0]]; // 上 右 下 左,4 是不动

function search(obs, goal) {
  const key = (x, y) => y * obs.width + x;
  const danger = new Set();
  if (obs.shield <= 2) for (const s of obs.sentries)
    for (const [dx, dy] of [[0, 0], ...DIRS]) danger.add(key(s.x + dx, s.y + dy));
  const queue = [[obs.x, obs.y, 4, 0]], seen = new Set([key(obs.x, obs.y)]);
  for (const [x, y, first, dist] of queue) {
    if (first !== 4 && goal(obs.cells[y][x], dist)) return first;
    DIRS.forEach(([dx, dy], d) => {
      const a = x + dx, b = y + dy, k = key(a, b);
      if (seen.has(k) || danger.has(k) || obs.cells[b][a] === 1) return;
      seen.add(k);
      queue.push([a, b, first === 4 ? d : first, dist + 1]);
    });
  }
  return -1;
}

function think(obs) {
  let d = search(obs, (c, dist) => c === 3 && dist <= 8);
  if (d < 0) d = search(obs, (c) => c >= 2);
  return { dir: d < 0 ? 4 : d };
}

npe.run(think);
