// 贪吃蛇:用洪水填充保证有路可走,再靠近果子。
#include "snake.h"
#include <algorithm>
#include <vector>

static const int DX[4] = {0, 1, 0, -1}, DY[4] = {-1, 0, 1, 0};

int think(const State *s) {
    std::vector<std::vector<int>> wall(s->height, std::vector<int>(s->width));
    for (int i = 0; i < s->snake_count; i++)
        if (s->snakes[i].alive)
            for (int k = 0; k < s->snakes[i].length; k++) wall[s->snakes[i].body[k].y][s->snakes[i].body[k].x] = 1;
    auto free_cell = [&](const std::vector<std::vector<int>> &g, int x, int y) {
        return x >= 0 && y >= 0 && x < s->width && y < s->height && !g[y][x];
    };
    const Snake &me = s->snakes[s->me];
    int best = -1000000, dir = UP;
    for (int d = 0; d < 4; d++) {
        int x = me.body[0].x + DX[d], y = me.body[0].y + DY[d];
        if (!free_cell(wall, x, y)) continue;
        auto g = wall;
        std::vector<Point> stack{{x, y}};
        g[y][x] = 1;
        int area = 0;
        while (!stack.empty()) {
            Point p = stack.back();
            stack.pop_back();
            area++;
            for (int k = 0; k < 4; k++)
                if (free_cell(g, p.x + DX[k], p.y + DY[k])) {
                    g[p.y + DY[k]][p.x + DX[k]] = 1;
                    stack.push_back({p.x + DX[k], p.y + DY[k]});
                }
        }
        int near = 999;
        for (int i = 0; i < s->food_count; i++) near = std::min(near, abs(s->food[i].x - x) + abs(s->food[i].y - y));
        int score = std::min(area, me.length * 3) * 20 - near;
        if (score > best) { best = score; dir = d; }
    }
    return dir;
}
