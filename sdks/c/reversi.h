/* 黑白棋。实现 Move think(const State *s)。 */
#ifndef NPE_REVERSI_H
#define NPE_REVERSI_H
#include "npe.h"

typedef struct { int x, y; } Move;
typedef struct {
    int size;          /* 棋盘边长,8 */
    int me, opponent;  /* 自己和对手的棋子编号,1 或 2 */
    int board[8][8];   /* board[y][x],0 表示空 */
    int move_count;    /* 合法落子的个数,至少为 1 */
    Move moves[64];    /* 全部合法落子 */
} State;

NPE_EXTERN Move think(const State *s);

static void npe_on_turn(const char *o, char *out, size_t size) {
    static State s;
    char path[32];
    int i;
    Move m;
    s.size = npe_int(o, "size", 8);
    s.me = npe_int(o, "me", 1);
    s.opponent = npe_int(o, "opponent", 2);
    npe_ints(o, "board", &s.board[0][0], 64);
    s.move_count = npe_len(o, "moves");
    if (s.move_count > 64) s.move_count = 64;
    for (i = 0; i < s.move_count; i++) {
        snprintf(path, sizeof path, "moves[%d].x", i);
        s.moves[i].x = npe_int(o, path, 0);
        snprintf(path, sizeof path, "moves[%d].y", i);
        s.moves[i].y = npe_int(o, path, 0);
    }
    m = think(&s);
    snprintf(out, size, "{\"x\":%d,\"y\":%d}", m.x, m.y);
}
#endif
