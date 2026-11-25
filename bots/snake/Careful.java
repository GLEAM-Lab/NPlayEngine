// 贪吃蛇:避开障碍,靠近最近的果子。
import java.util.*;

public class Careful {
    static final int[] DX = {0, 1, 0, -1}, DY = {-1, 0, 1, 0};

    public static void main(String[] args) {
        Npe.run(obs -> {
            int w = obs.getInt("width"), h = obs.getInt("height"), me = obs.getInt("me");
            boolean[][] wall = new boolean[h][w];
            Npe.Json snakes = obs.get("snakes");
            for (int i = 0; i < snakes.size(); i++)
                for (int[] c : snakes.get(i).getGrid("body")) wall[c[1]][c[0]] = true;
            int[] head = snakes.get(me).getGrid("body")[0];
            int[][] food = obs.getGrid("food");
            int best = Integer.MIN_VALUE, dir = 0;
            for (int d = 0; d < 4; d++) {
                int x = head[0] + DX[d], y = head[1] + DY[d];
                if (x < 0 || y < 0 || x >= w || y >= h || wall[y][x]) continue;
                int near = 999;
                for (int[] f : food) near = Math.min(near, Math.abs(f[0] - x) + Math.abs(f[1] - y));
                int exits = 0;
                for (int k = 0; k < 4; k++) {
                    int a = x + DX[k], b = y + DY[k];
                    if (a >= 0 && b >= 0 && a < w && b < h && !wall[b][a]) exits++;
                }
                int score = exits * 6 - near;
                if (score > best) { best = score; dir = d; }
            }
            return Npe.action("dir", dir);
        });
    }
}
