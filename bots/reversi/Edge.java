// 黑白棋:优先占角和边。
public class Edge {
    static int value(int x, int y) {
        boolean ex = x == 0 || x == 7, ey = y == 0 || y == 7;
        return ex && ey ? 100 : ex || ey ? 10 : 1;
    }

    public static void main(String[] args) {
        Npe.run(obs -> {
            int bx = 0, by = 0, best = -1;
            Npe.Json moves = obs.get("moves");
            for (int i = 0; i < moves.size(); i++) {
                int x = moves.get(i).getInt("x"), y = moves.get(i).getInt("y");
                if (value(x, y) > best) { best = value(x, y); bx = x; by = y; }
            }
            return Npe.action("x", bx, "y", by);
        });
    }
}
