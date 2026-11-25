/* 迷宫拾光:广度优先搜索最近的光点或护盾。 */
#include "maze.h"

static const int DX[4] = {0, 1, 0, -1}, DY[4] = {-1, 0, 1, 0};

static int danger(const State *s, int x, int y) {
    if (s->shield > 2) return 0;
    for (int i = 0; i < s->sentry_count; i++)
        if (abs(s->sentries[i].x - x) + abs(s->sentries[i].y - y) <= 1) return 1;
    return 0;
}

int think(const State *s) {
    static int qx[315], qy[315], first[315];
    int seen[15][21] = {{0}}, head = 0, tail = 0;
    qx[0] = s->x; qy[0] = s->y; first[0] = STAY; tail = 1;
    seen[s->y][s->x] = 1;
    while (head < tail) {
        int cx = qx[head], cy = qy[head], f = first[head];
        head++;
        if (f != STAY && s->cells[cy][cx] >= LIGHT) return f;
        for (int d = 0; d < 4; d++) {
            int x = cx + DX[d], y = cy + DY[d];
            if (s->cells[y][x] == WALL || seen[y][x] || danger(s, x, y)) continue;
            seen[y][x] = 1;
            qx[tail] = x; qy[tail] = y; first[tail] = f == STAY ? d : f;
            tail++;
        }
    }
    return STAY;
}
