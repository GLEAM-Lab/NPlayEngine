/* 贪吃蛇。实现 int think(const State *s),返回 UP、RIGHT、DOWN 或 LEFT。 */
#ifndef NPE_SNAKE_H
#define NPE_SNAKE_H
#include "npe.h"

enum { UP, RIGHT, DOWN, LEFT };
typedef struct { int x, y; } Point;
typedef struct {
    int alive, dir, length;
    Point body[432];   /* body[0] 是蛇头 */
} Snake;
typedef struct {
    int width, height, turn;
    int me;            /* 自己在 snakes 里的下标 */
    int snake_count;
    Snake snakes[6];
    int food_count;
    Point food[32];
} State;

NPE_EXTERN int think(const State *s);

static void npe_on_turn(const char *o, char *out, size_t size) {
    static State s;
    static int raw[864];
    char path[32];
    int i, k, n;
    s.width = npe_int(o, "width", 0);
    s.height = npe_int(o, "height", 0);
    s.turn = npe_int(o, "turn", 0);
    s.me = npe_int(o, "me", 0);
    s.snake_count = npe_len(o, "snakes");
    if (s.snake_count > 6) s.snake_count = 6;
    for (i = 0; i < s.snake_count; i++) {
        Snake *q = &s.snakes[i];
        snprintf(path, sizeof path, "snakes[%d]", i);
        {
            const char *v = npe_get(o, path);
            q->alive = npe_int(v, "alive", 0);
            q->dir = npe_int(v, "dir", 0);
            n = npe_ints(v, "body", raw, 864);
            q->length = n / 2;
            for (k = 0; k < q->length; k++) { q->body[k].x = raw[2 * k]; q->body[k].y = raw[2 * k + 1]; }
        }
    }
    n = npe_ints(o, "food", raw, 64);
    s.food_count = n / 2;
    for (k = 0; k < s.food_count; k++) { s.food[k].x = raw[2 * k]; s.food[k].y = raw[2 * k + 1]; }
    snprintf(out, size, "{\"dir\":%d}", think(&s));
}
#endif
