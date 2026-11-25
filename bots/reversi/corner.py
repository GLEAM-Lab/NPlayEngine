# 黑白棋:优先占角,其次占边,避开角旁边的格子。
import npe


def value(x, y):
    edge_x = x in (0, 7)
    edge_y = y in (0, 7)
    if edge_x and edge_y:
        return 100
    if x in (0, 1, 6, 7) and y in (0, 1, 6, 7):
        return -30
    return 10 if edge_x or edge_y else 1


def think(obs):
    # obs["moves"] 是所有合法落子,obs["board"][y][x] 是棋盘
    return max(obs["moves"], key=lambda m: value(m["x"], m["y"]))


npe.run(think)
