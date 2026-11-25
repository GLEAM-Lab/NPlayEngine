/* NPlayEngine C/C++ SDK. One header, no dependencies.
 *
 * Students include a game header (reversi.h, snake.h, maze.h) and write one
 * function. Everything printed with printf goes to the debug log: the real
 * stdout is moved away before student code runs, so it cannot break the
 * protocol.
 *
 * Built with a wasm32-wasi target, the same code exports npe_buf and npe_line
 * instead of main, because a browser has no blocking stdin.
 */
#ifndef NPE_H
#define NPE_H
#ifndef _CRT_SECURE_NO_WARNINGS
#define _CRT_SECURE_NO_WARNINGS
#endif
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* ---------- tiny JSON reader, no allocation ---------- */

static const char *npe_ws(const char *p) {
    while (*p == ' ' || *p == '\t' || *p == '\r' || *p == '\n' || *p == ',' || *p == ':') p++;
    return p;
}

/* Pointer just past the value at p. */
static const char *npe_skip(const char *p) {
    p = npe_ws(p);
    if (*p == '"') {
        for (p++; *p && *p != '"'; p++)
            if (*p == '\\' && p[1]) p++;
        return *p ? p + 1 : p;
    }
    if (*p == '{' || *p == '[') {
        int depth = 0;
        while (*p) {
            if (*p == '"') { p = npe_skip(p); continue; }
            if (*p == '{' || *p == '[') depth++;
            if (*p == '}' || *p == ']') { if (--depth == 0) return p + 1; }
            p++;
        }
        return p;
    }
    while (*p && *p != ',' && *p != ']' && *p != '}') p++;
    return p;
}

/* Find a value by path, for example "snakes[2].body[0][1]". NULL if absent. */
static const char *npe_get(const char *v, const char *path) {
    while (v && *path) {
        v = npe_ws(v);
        if (*path == '.') path++;
        if (*path == '[') {
            int k = atoi(path + 1);
            path = strchr(path, ']');
            if (!path || *v != '[') return NULL;
            path++;
            v = npe_ws(v + 1);
            while (k-- > 0 && *v && *v != ']') v = npe_ws(npe_skip(v));
            if (!*v || *v == ']') return NULL;
        } else {
            size_t n = strcspn(path, ".[");
            const char *found = NULL;
            if (*v != '{') return NULL;
            v = npe_ws(v + 1);
            while (*v == '"') {
                const char *end = npe_skip(v);
                int hit = (size_t)(end - v) == n + 2 && strncmp(v + 1, path, n) == 0;
                v = npe_ws(end);
                if (hit) { found = v; break; }
                v = npe_ws(npe_skip(v));
            }
            v = found;
            path += n;
        }
    }
    return v ? npe_ws(v) : NULL;
}

static int npe_int(const char *v, const char *path, int otherwise) {
    const char *p = npe_get(v, path);
    if (!p) return otherwise;
    if (!strncmp(p, "true", 4)) return 1;
    if (!strncmp(p, "false", 5)) return 0;
    if (*p != '-' && (*p < '0' || *p > '9')) return otherwise;
    return atoi(p);
}

static int npe_len(const char *v, const char *path) {
    const char *p = npe_get(v, path);
    int k = 0;
    if (!p || *p != '[') return 0;
    for (p = npe_ws(p + 1); *p && *p != ']'; p = npe_ws(npe_skip(p))) k++;
    return k;
}

/* Read every integer inside an array, nested or not. Returns how many. */
static int npe_ints(const char *v, const char *path, int *out, int max) {
    const char *p = npe_get(v, path), *end;
    int k = 0;
    if (!p || *p != '[') return 0;
    end = npe_skip(p);
    while (p < end && k < max) {
        if (*p == '-' || (*p >= '0' && *p <= '9')) {
            out[k++] = atoi(p);
            while (*p == '-' || (*p >= '0' && *p <= '9')) p++;
        } else p++;
    }
    return k;
}

/* ---------- protocol ---------- */

/* Defined by the game header: read obs, call think(), write the action JSON. */
static void npe_on_turn(const char *obs, char *out, size_t size);

/* Returns 1 when a reply is ready in out. */
static int npe_handle(const char *line, char *out, size_t size) {
    const char *type = npe_get(line, "type");
    char action[200];
    if (!type) return 0;
    if (!strncmp(type, "\"init\"", 6)) {
        snprintf(out, size, "{\"ready\":true}");
        return 1;
    }
    if (!strncmp(type, "\"turn\"", 6)) {
        const char *obs = npe_get(line, "obs");
        if (!obs) return 0;
        npe_on_turn(obs, action, sizeof action);
        snprintf(out, size, "{\"turn\":%d,\"action\":%s}", npe_int(line, "turn", -1), action);
        return 1;
    }
    return 0;
}

#ifdef __cplusplus
#define NPE_EXTERN extern "C"
#else
#define NPE_EXTERN
#endif

#ifdef __wasm__
static char *npe_line_buf;
static char npe_reply[256];
NPE_EXTERN __attribute__((export_name("npe_buf"))) char *npe_buf(int len) {
    npe_line_buf = (char *)realloc(npe_line_buf, (size_t)len + 1);
    if (npe_line_buf) npe_line_buf[len] = 0;
    return npe_line_buf;
}
/* Handles the line in npe_buf. Returns a C string, "" when there is no reply. */
NPE_EXTERN __attribute__((export_name("npe_line"))) const char *npe_line(void) {
    npe_reply[0] = 0;
    npe_handle(npe_line_buf, npe_reply, sizeof npe_reply);
    fflush(stdout);
    return npe_reply;
}
#else
#ifdef _WIN32
#include <io.h>
#define npe_dup _dup
#define npe_dup2 _dup2
#define npe_fdopen _fdopen
#else
#include <unistd.h>
#define npe_dup dup
#define npe_dup2 dup2
#define npe_fdopen fdopen
#endif
int main(void) {
    char reply[256];
    size_t cap = 1 << 16, len = 0;
    char *line = (char *)malloc(cap);
    FILE *wire;
    fflush(stdout);
    wire = npe_fdopen(npe_dup(1), "w");
    npe_dup2(2, 1);
    setvbuf(stdout, NULL, _IONBF, 0);
    while (line && fgets(line + len, (int)(cap - len), stdin)) {
        len += strlen(line + len);
        if (len == 0 || line[len - 1] != '\n') {
            if (len + 1 >= cap) { cap *= 2; line = (char *)realloc(line, cap); }
            continue;
        }
        len = 0;
        if (npe_handle(line, reply, sizeof reply)) {
            fprintf(wire, "%s\n", reply);
            fflush(wire);
        }
    }
    free(line);
    return 0;
}
#endif
#endif
