// 迷宫拾光:广度优先搜索最近的光点或护盾。
#include "maze.h"
#include <queue>

static const int DX[4] = {0, 1, 0, -1}, DY[4] = {-1, 0, 1, 0};

static int danger(const State *s, int x, int y) {
    if (s->shield > 2) return 0;
    for (int i = 0; i < s->sentry_count; i++)
        if (abs(s->sentries[i].x - x) + abs(s->sentries[i].y - y) <= 2) return 1;
    return 0;
}

int think(const State *s) {
    struct Node { int x, y, first; };
    std::queue<Node> q;
    bool seen[15][21] = {};
    q.push({s->x, s->y, STAY});
    seen[s->y][s->x] = true;
    while (!q.empty()) {
        Node n = q.front();
        q.pop();
        if (n.first != STAY && s->cells[n.y][n.x] >= LIGHT) return n.first;
        for (int d = 0; d < 4; d++) {
            int x = n.x + DX[d], y = n.y + DY[d];
            if (s->cells[y][x] == WALL || seen[y][x] || danger(s, x, y)) continue;
            seen[y][x] = true;
            q.push({x, y, n.first == STAY ? d : n.first});
        }
    }
    return STAY;
}
