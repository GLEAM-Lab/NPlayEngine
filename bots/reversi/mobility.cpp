// 黑白棋:走完一步后,让对手能走的地方尽量少。
#include "reversi.h"
#include <array>
#include <vector>

using Board = std::array<std::array<int, 8>, 8>;

static std::vector<Move> flips(const Board &b, int x, int y, int me) {
    std::vector<Move> all;
    if (b[y][x]) return all;
    for (int dx = -1; dx <= 1; dx++)
        for (int dy = -1; dy <= 1; dy++) {
            if (!dx && !dy) continue;
            std::vector<Move> line;
            int i = x + dx, j = y + dy;
            while (i >= 0 && j >= 0 && i < 8 && j < 8 && b[j][i] == 3 - me) {
                line.push_back({i, j});
                i += dx;
                j += dy;
            }
            if (i >= 0 && j >= 0 && i < 8 && j < 8 && b[j][i] == me) all.insert(all.end(), line.begin(), line.end());
        }
    return all;
}

Move think(const State *s) {
    Board board;
    for (int y = 0; y < 8; y++)
        for (int x = 0; x < 8; x++) board[y][x] = s->board[y][x];
    Move best = s->moves[0];
    int best_score = -1000000;
    for (int k = 0; k < s->move_count; k++) {
        Move m = s->moves[k];
        Board b = board;
        for (Move f : flips(board, m.x, m.y, s->me)) b[f.y][f.x] = s->me;
        b[m.y][m.x] = s->me;
        int options = 0;
        for (int y = 0; y < 8; y++)
            for (int x = 0; x < 8; x++)
                if (!flips(b, x, y, s->opponent).empty()) options++;
        bool corner = (m.x == 0 || m.x == 7) && (m.y == 0 || m.y == 7);
        int score = -options + (corner ? 100 : 0);
        if (score > best_score) { best_score = score; best = m; }
    }
    return best;
}
