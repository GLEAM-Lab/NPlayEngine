/* 迷宫拾光。实现 int think(const State *s),返回 UP、RIGHT、DOWN、LEFT 或 STAY。 */
#ifndef NPE_MAZE_H
#define NPE_MAZE_H
#include "npe.h"

enum { UP, RIGHT, DOWN, LEFT, STAY };
enum { EMPTY, WALL, LIGHT, SHIELD };
typedef struct { int x, y; } Point;
typedef struct {
    int width, height, turn;
    int me, x, y;
    int shield;            /* 护盾剩余回合 */
    int lives;
    int cells[15][21];     /* cells[y][x] */
    int sentry_count;      /* 视野内的哨兵 */
    Point sentries[8];
} State;

NPE_EXTERN int think(const State *s);

static void npe_on_turn(const char *o, char *out, size_t size) {
    static State s;
    char path[32];
    int i;
    s.width = npe_int(o, "width", 21);
    s.height = npe_int(o, "height", 15);
    s.turn = npe_int(o, "turn", 0);
    s.me = npe_int(o, "me", 0);
    s.x = npe_int(o, "x", 0);
    s.y = npe_int(o, "y", 0);
    s.shield = npe_int(o, "shield", 0);
    s.lives = npe_int(o, "lives", 0);
    npe_ints(o, "cells", &s.cells[0][0], 15 * 21);
    s.sentry_count = npe_len(o, "sentries");
    if (s.sentry_count > 8) s.sentry_count = 8;
    for (i = 0; i < s.sentry_count; i++) {
        snprintf(path, sizeof path, "sentries[%d].x", i);
        s.sentries[i].x = npe_int(o, path, 0);
        snprintf(path, sizeof path, "sentries[%d].y", i);
        s.sentries[i].y = npe_int(o, path, 0);
    }
    snprintf(out, size, "{\"dir\":%d}", think(&s));
}
#endif
