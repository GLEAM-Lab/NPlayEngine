/* 黑白棋:每个格子有一个分值,选分值最高的合法落子。 */
#include "reversi.h"

static const int WEIGHT[8][8] = {
    {100, -20, 10, 5, 5, 10, -20, 100},
    {-20, -50, -2, -2, -2, -2, -50, -20},
    {10, -2, 3, 1, 1, 3, -2, 10},
    {5, -2, 1, 0, 0, 1, -2, 5},
    {5, -2, 1, 0, 0, 1, -2, 5},
    {10, -2, 3, 1, 1, 3, -2, 10},
    {-20, -50, -2, -2, -2, -2, -50, -20},
    {100, -20, 10, 5, 5, 10, -20, 100},
};

Move think(const State *s) {
    Move best = s->moves[0];
    for (int i = 1; i < s->move_count; i++) {
        Move m = s->moves[i];
        if (WEIGHT[m.y][m.x] > WEIGHT[best.y][best.x]) best = m;
    }
    printf("play %d %d\n", best.x, best.y); /* printf 只写日志,不影响对战 */
    return best;
}
