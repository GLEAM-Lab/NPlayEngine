// 贪吃蛇:走向空间最大的方向,空间一样大时靠近果子。
const DIRS = [[0, -1], [1, 0], [0, 1], [-1, 0]]; // 上 右 下 左

function think(obs) {
  const key = (x, y) => y * obs.width + x;
  const blocked = new Set();
  for (const s of obs.snakes) if (s.alive) for (const [x, y] of s.body) blocked.add(key(x, y));
  const free = (x, y) => x >= 0 && y >= 0 && x < obs.width && y < obs.height && !blocked.has(key(x, y));
  const area = (x, y) => {
    const seen = new Set([key(x, y)]), stack = [[x, y]];
    while (stack.length) {
      const [a, b] = stack.pop();
      for (const [dx, dy] of DIRS) {
        const k = key(a + dx, b + dy);
        if (free(a + dx, b + dy) && !seen.has(k)) { seen.add(k); stack.push([a + dx, b + dy]); }
      }
    }
    return seen.size;
  };
  const [hx, hy] = obs.snakes[obs.me].body[0];
  let best = -Infinity, dir = 0;
  DIRS.forEach(([dx, dy], d) => {
    const x = hx + dx, y = hy + dy;
    if (!free(x, y)) return;
    const near = Math.min(99, ...obs.food.map(([fx, fy]) => Math.abs(fx - x) + Math.abs(fy - y)));
    const score = area(x, y) * 10 - near;
    if (score > best) { best = score; dir = d; }
  });
  return { dir };
}

npe.run(think);
