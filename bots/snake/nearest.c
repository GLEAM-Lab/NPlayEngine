/* 贪吃蛇:不撞墙不撞蛇,尽量靠近最近的果子。 */
#include "snake.h"

static const int DX[4] = {0, 1, 0, -1}, DY[4] = {-1, 0, 1, 0};

static int blocked(const State *s, int x, int y) {
    if (x < 0 || y < 0 || x >= s->width || y >= s->height) return 1;
    for (int i = 0; i < s->snake_count; i++) {
        if (!s->snakes[i].alive) continue;
        for (int k = 0; k < s->snakes[i].length; k++)
            if (s->snakes[i].body[k].x == x && s->snakes[i].body[k].y == y) return 1;
    }
    return 0;
}

int think(const State *s) {
    Point head = s->snakes[s->me].body[0];
    int best = -100000, dir = UP;
    for (int d = 0; d < 4; d++) {
        int x = head.x + DX[d], y = head.y + DY[d], score = 0, near = 999, i;
        if (blocked(s, x, y)) continue;
        for (i = 0; i < 4; i++) if (!blocked(s, x + DX[i], y + DY[i])) score += 5;
        for (i = 0; i < s->food_count; i++) {
            int dist = abs(s->food[i].x - x) + abs(s->food[i].y - y);
            if (dist < near) near = dist;
        }
        score -= near;
        if (score > best) { best = score; dir = d; }
    }
    return dir;
}
