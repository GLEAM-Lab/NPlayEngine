# 贪吃蛇:用广度优先搜索走向最近的果子,走不到就找一个空格子。
from collections import deque
import npe

DIRS = [(0, -1), (1, 0), (0, 1), (-1, 0)]  # 上 右 下 左


def think(obs):
    w, h = obs["width"], obs["height"]
    blocked = {tuple(c) for s in obs["snakes"] if s["alive"] for c in s["body"]}
    food = {tuple(f) for f in obs["food"]}
    head = tuple(obs["snakes"][obs["me"]]["body"][0])
    queue = deque([(head, None)])
    seen = {head}
    fallback = None
    while queue:
        (x, y), first = queue.popleft()
        if (x, y) in food and first is not None:
            return {"dir": first}
        for d, (dx, dy) in enumerate(DIRS):
            c = (x + dx, y + dy)
            if c in seen or c in blocked or not (0 <= c[0] < w and 0 <= c[1] < h):
                continue
            seen.add(c)
            step = d if first is None else first
            if fallback is None:
                fallback = step
            queue.append((c, step))
    return {"dir": fallback if fallback is not None else 0}


npe.run(think)
