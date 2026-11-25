import java.io.*;
import java.nio.charset.StandardCharsets;
import java.util.*;
import java.util.function.Function;

/** NPlayEngine Java SDK: call Npe.run(obs -> Npe.action("dir", 1)). */
public final class Npe {
    /** A JSON value. Missing paths return defaults instead of throwing. */
    public static final class Json {
        private final Object v;
        Json(Object v) { this.v = v; }

        /** Path lookup, for example get("snakes[2].body[0][1]"). */
        public Json get(String path) {
            Object cur = v;
            for (String part : path.replace("[", ".").replace("]", "").split("\\.")) {
                if (part.isEmpty()) continue;
                if (cur instanceof Map) cur = ((Map<?, ?>) cur).get(part);
                else if (cur instanceof List) cur = at((List<?>) cur, Integer.parseInt(part));
                else return new Json(null);
            }
            return new Json(cur);
        }
        public Json get(int i) { return new Json(v instanceof List ? at((List<?>) v, i) : null); }
        private static Object at(List<?> l, int i) { return i >= 0 && i < l.size() ? l.get(i) : null; }
        public int size() { return v instanceof List ? ((List<?>) v).size() : 0; }
        public boolean isNull() { return v == null; }
        public int asInt() { return v instanceof Double ? ((Double) v).intValue() : 0; }
        public double asDouble() { return v instanceof Double ? (Double) v : 0; }
        public boolean asBool() { return Boolean.TRUE.equals(v); }
        public String asString() { return v == null ? "" : v.toString(); }
        public int getInt(String path) { return get(path).asInt(); }
        public double getDouble(String path) { return get(path).asDouble(); }
        public boolean getBool(String path) { return get(path).asBool(); }
        public int[][] getGrid(String path) {
            Json g = get(path);
            int[][] r = new int[g.size()][];
            for (int y = 0; y < r.length; y++) {
                r[y] = new int[g.get(y).size()];
                for (int x = 0; x < r[y].length; x++) r[y][x] = g.get(y).get(x).asInt();
            }
            return r;
        }
    }

    private static final class Parser {
        private final String s;
        private int i = 0;
        Parser(String s) { this.s = s; }
        private void ws() { while (i < s.length() && " \t\r\n,:".indexOf(s.charAt(i)) >= 0) i++; }
        Object value() {
            ws();
            char c = s.charAt(i);
            if (c == '{') {
                Map<String, Object> m = new LinkedHashMap<>();
                i++;
                for (ws(); s.charAt(i) != '}'; ws()) { String k = string(); m.put(k, value()); }
                i++;
                return m;
            }
            if (c == '[') {
                List<Object> l = new ArrayList<>();
                i++;
                for (ws(); s.charAt(i) != ']'; ws()) l.add(value());
                i++;
                return l;
            }
            if (c == '"') return string();
            int b = i;
            while (i < s.length() && ",]} \r\n".indexOf(s.charAt(i)) < 0) i++;
            String t = s.substring(b, i);
            switch (t) {
                case "true": return true;
                case "false": return false;
                case "null": return null;
                default: return Double.valueOf(t);
            }
        }
        private String string() {
            StringBuilder b = new StringBuilder();
            for (i++; s.charAt(i) != '"'; i++) {
                char c = s.charAt(i);
                if (c == '\\') {
                    c = s.charAt(++i);
                    if (c == 'u') { c = (char) Integer.parseInt(s.substring(i + 1, i + 5), 16); i += 4; }
                    else if (c == 'n') c = '\n';
                    else if (c == 't') c = '\t';
                }
                b.append(c);
            }
            i++;
            return b.toString();
        }
    }

    /** Build an action from name and value pairs. */
    public static String action(Object... kv) {
        StringBuilder b = new StringBuilder("{");
        for (int k = 0; k + 1 < kv.length; k += 2) {
            if (k > 0) b.append(',');
            b.append('"').append(kv[k]).append("\":").append(kv[k + 1]);
        }
        return b.append('}').toString();
    }

    public static void run(Function<Json, String> think) {
        PrintStream wire = System.out;
        System.setOut(System.err); // println goes to the debug log
        try (BufferedReader r = new BufferedReader(new InputStreamReader(System.in, StandardCharsets.UTF_8))) {
            for (String line; (line = r.readLine()) != null; ) {
                Json m = new Json(new Parser(line).value());
                String type = m.get("type").asString(), out;
                if (type.equals("init")) out = "{\"ready\":true}";
                else if (type.equals("turn")) out = "{\"turn\":" + m.getInt("turn") + ",\"action\":" + think.apply(m.get("obs")) + "}";
                else continue;
                wire.print(out + "\n");
                wire.flush();
            }
        } catch (IOException e) {
            System.exit(1);
        }
    }
}
