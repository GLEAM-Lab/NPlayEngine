// 迷宫拾光:广度优先搜索最近的光点。
import java.util.*;

public class Seeker {
    static final int[] DX = {0, 1, 0, -1}, DY = {-1, 0, 1, 0};

    public static void main(String[] args) {
        Npe.run(obs -> {
            int[][] cells = obs.getGrid("cells");
            boolean[][] seen = new boolean[cells.length][cells[0].length];
            Npe.Json sentries = obs.get("sentries");
            if (obs.getInt("shield") <= 2)
                for (int i = 0; i < sentries.size(); i++) {
                    int x = sentries.get(i).getInt("x"), y = sentries.get(i).getInt("y");
                    seen[y][x] = true;
                    for (int d = 0; d < 4; d++) seen[y + DY[d]][x + DX[d]] = true;
                }
            ArrayDeque<int[]> q = new ArrayDeque<>();
            q.add(new int[] {obs.getInt("x"), obs.getInt("y"), 4});
            while (!q.isEmpty()) {
                int[] n = q.poll();
                if (n[2] != 4 && cells[n[1]][n[0]] >= 2) return Npe.action("dir", n[2]);
                for (int d = 0; d < 4; d++) {
                    int x = n[0] + DX[d], y = n[1] + DY[d];
                    if (cells[y][x] == 1 || seen[y][x]) continue;
                    seen[y][x] = true;
                    q.add(new int[] {x, y, n[2] == 4 ? d : n[2]});
                }
            }
            return Npe.action("dir", 4);
        });
    }
}
