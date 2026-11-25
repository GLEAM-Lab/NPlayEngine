# 迷宫拾光:走向最近的光点,绕开看得见的哨兵。
from collections import deque
import npe

DIRS = [(0, -1), (1, 0), (0, 1), (-1, 0)]  # 上 右 下 左,4 是不动


def think(obs):
    cells = obs["cells"]
    danger = set()
    if obs["shield"] <= 2:
        for s in obs["sentries"]:
            danger.add((s["x"], s["y"]))
            for dx, dy in DIRS:
                danger.add((s["x"] + dx, s["y"] + dy))
    start = (obs["x"], obs["y"])
    queue = deque([(start, 4)])
    seen = {start}
    while queue:
        (x, y), first = queue.popleft()
        if cells[y][x] >= 2 and first != 4:
            return {"dir": first}
        for d, (dx, dy) in enumerate(DIRS):
            c = (x + dx, y + dy)
            if c in seen or c in danger or cells[c[1]][c[0]] == 1:
                continue
            seen.add(c)
            queue.append((c, d if first == 4 else first))
    return {"dir": 4}


npe.run(think)
