/* Host I/O Runtime substrate (H00.02–H00.03, H01 process, H02.01 stdout).
   Error codes, opaque handles, UTF-8 path encoding, I/O bytes boundary,
   process user-args + env + exit + pid/ppid, stdout write. Later H rows. */

#include "draconic_rt_host.h"

#include <errno.h>
#include <fcntl.h>
#include <stdio.h>
#include <stdint.h>
#include <stdlib.h>
#include <string.h>
#include <sys/stat.h>
#include <time.h>

#if defined(_WIN32)
#include <direct.h>
#include <io.h>
#include <process.h>
#include <tlhelp32.h>
#include <windows.h>
/* getenv / _putenv_s; _getpid; _mkdir / _rmdir / _unlink; _open/_read/_write/_lseeki64/_close */
#else
#include <dirent.h>
#include <sys/time.h>
#include <unistd.h>
/* setenv / unsetenv; getpid / getppid; mkdir / rmdir / unlink; open/read/write/lseek/close */
#endif

/* --- Handle table (slots filled by open/listen/etc.) --- */

#define DRACONIC_HOST_HANDLE_SLOTS 256
#define DRACONIC_HOST_HANDLE_KIND_NONE 0
#define DRACONIC_HOST_HANDLE_KIND_FILE 1

/* Live flags + kind + OS fd for 1-based handle ids. */
static uint8_t g_host_handle_live[DRACONIC_HOST_HANDLE_SLOTS];
static uint8_t g_host_handle_kind[DRACONIC_HOST_HANDLE_SLOTS];
static int g_host_handle_fd[DRACONIC_HOST_HANDLE_SLOTS];

static DraconicHostError host_handle_alloc(
    uint8_t kind,
    int fd,
    DraconicHostHandle *out_h) {
    size_t i;
    if (!out_h) {
        return DRACONIC_HOST_E_INVAL;
    }
    for (i = 0; i < DRACONIC_HOST_HANDLE_SLOTS; i++) {
        if (g_host_handle_live[i] == 0) {
            g_host_handle_live[i] = 1;
            g_host_handle_kind[i] = kind;
            g_host_handle_fd[i] = fd;
            *out_h = (DraconicHostHandle)(i + 1);
            return DRACONIC_HOST_OK;
        }
    }
    return DRACONIC_HOST_E_NOMEM;
}

int draconic_rt_host_handle_is_valid(DraconicHostHandle h) {
    if (h < 1 || h > (DraconicHostHandle)DRACONIC_HOST_HANDLE_SLOTS) {
        return 0;
    }
    return g_host_handle_live[(size_t)h - 1] != 0;
}

static int host_handle_fd(DraconicHostHandle h) {
    if (!draconic_rt_host_handle_is_valid(h)) {
        return -1;
    }
    if (g_host_handle_kind[(size_t)h - 1] != DRACONIC_HOST_HANDLE_KIND_FILE) {
        return -1;
    }
    return g_host_handle_fd[(size_t)h - 1];
}

DraconicHostError draconic_rt_host_handle_close(DraconicHostHandle h) {
    size_t i;
    if (!draconic_rt_host_handle_is_valid(h)) {
        return DRACONIC_HOST_E_BADF;
    }
    i = (size_t)h - 1;
    if (g_host_handle_kind[i] == DRACONIC_HOST_HANDLE_KIND_FILE) {
        int fd = g_host_handle_fd[i];
        if (fd >= 0) {
#if defined(_WIN32)
            (void)_close(fd);
#else
            (void)close(fd);
#endif
        }
    }
    g_host_handle_live[i] = 0;
    g_host_handle_kind[i] = DRACONIC_HOST_HANDLE_KIND_NONE;
    g_host_handle_fd[i] = -1;
    return DRACONIC_HOST_OK;
}

/* --- UTF-8 validation (RFC 3629 well-formed; no overlongs / surrogates) --- */

static int host_utf8_is_valid(const unsigned char *s, size_t len) {
    size_t i = 0;
    while (i < len) {
        unsigned char c = s[i];
        if (c <= 0x7Fu) {
            i += 1;
            continue;
        }
        if (c >= 0xC2u && c <= 0xDFu) {
            if (i + 1 >= len) {
                return 0;
            }
            if ((s[i + 1] & 0xC0u) != 0x80u) {
                return 0;
            }
            i += 2;
            continue;
        }
        if (c == 0xE0u) {
            if (i + 2 >= len) {
                return 0;
            }
            if (s[i + 1] < 0xA0u || s[i + 1] > 0xBFu) {
                return 0;
            }
            if ((s[i + 2] & 0xC0u) != 0x80u) {
                return 0;
            }
            i += 3;
            continue;
        }
        if (c >= 0xE1u && c <= 0xECu) {
            if (i + 2 >= len) {
                return 0;
            }
            if ((s[i + 1] & 0xC0u) != 0x80u || (s[i + 2] & 0xC0u) != 0x80u) {
                return 0;
            }
            i += 3;
            continue;
        }
        if (c == 0xEDu) {
            /* U+D800..U+DFFF surrogates are invalid in UTF-8. */
            if (i + 2 >= len) {
                return 0;
            }
            if (s[i + 1] < 0x80u || s[i + 1] > 0x9Fu) {
                return 0;
            }
            if ((s[i + 2] & 0xC0u) != 0x80u) {
                return 0;
            }
            i += 3;
            continue;
        }
        if (c >= 0xEEu && c <= 0xEFu) {
            if (i + 2 >= len) {
                return 0;
            }
            if ((s[i + 1] & 0xC0u) != 0x80u || (s[i + 2] & 0xC0u) != 0x80u) {
                return 0;
            }
            i += 3;
            continue;
        }
        if (c == 0xF0u) {
            if (i + 3 >= len) {
                return 0;
            }
            if (s[i + 1] < 0x90u || s[i + 1] > 0xBFu) {
                return 0;
            }
            if ((s[i + 2] & 0xC0u) != 0x80u || (s[i + 3] & 0xC0u) != 0x80u) {
                return 0;
            }
            i += 4;
            continue;
        }
        if (c >= 0xF1u && c <= 0xF3u) {
            if (i + 3 >= len) {
                return 0;
            }
            if ((s[i + 1] & 0xC0u) != 0x80u
                || (s[i + 2] & 0xC0u) != 0x80u
                || (s[i + 3] & 0xC0u) != 0x80u) {
                return 0;
            }
            i += 4;
            continue;
        }
        if (c == 0xF4u) {
            if (i + 3 >= len) {
                return 0;
            }
            if (s[i + 1] < 0x80u || s[i + 1] > 0x8Fu) {
                return 0;
            }
            if ((s[i + 2] & 0xC0u) != 0x80u || (s[i + 3] & 0xC0u) != 0x80u) {
                return 0;
            }
            i += 4;
            continue;
        }
        return 0;
    }
    return 1;
}

DraconicHostError draconic_rt_host_path_from_utf8(
    const char *utf8,
    size_t len,
    char **out_path) {
    size_t i;
    char *buf;

    if (!out_path) {
        return DRACONIC_HOST_E_INVAL;
    }
    *out_path = NULL;

    if (len > 0 && !utf8) {
        return DRACONIC_HOST_E_INVAL;
    }

    /* OS paths are C strings: no embedded NUL before the terminator. */
    if (utf8) {
        for (i = 0; i < len; i++) {
            if (utf8[i] == '\0') {
                return DRACONIC_HOST_E_INVAL;
            }
        }
    }

    if (len > 0 && !host_utf8_is_valid((const unsigned char *)utf8, len)) {
        return DRACONIC_HOST_E_INVAL;
    }

    buf = (char *)malloc(len + 1);
    if (!buf) {
        return DRACONIC_HOST_E_NOMEM;
    }
    if (len > 0) {
        memcpy(buf, utf8, len);
    }
    buf[len] = '\0';
    *out_path = buf;
    return DRACONIC_HOST_OK;
}

void draconic_rt_host_path_free(char *path) {
    free(path);
}

/* --- I/O bytes boundary (H00.03): ArrayBuffer / Uint8Array OS buffers --- */

DraconicHostError draconic_rt_host_bytes_from_raw(
    uint8_t *data,
    size_t len,
    DraconicHostBytes *out) {
    if (!out) {
        return DRACONIC_HOST_E_INVAL;
    }
    if (len > 0 && !data) {
        out->data = NULL;
        out->len = 0;
        return DRACONIC_HOST_E_INVAL;
    }
    out->data = data;
    out->len = len;
    return DRACONIC_HOST_OK;
}

DraconicHostError draconic_rt_host_bytes_view(
    const DraconicHostBytes *parent,
    size_t byte_offset,
    size_t byte_length,
    DraconicHostBytes *out) {
    size_t end;

    if (!out) {
        return DRACONIC_HOST_E_INVAL;
    }
    out->data = NULL;
    out->len = 0;

    if (!parent) {
        return DRACONIC_HOST_E_INVAL;
    }
    if (parent->len > 0 && !parent->data) {
        return DRACONIC_HOST_E_INVAL;
    }

    /* offset + length must not overflow or exceed parent. */
    if (byte_offset > parent->len) {
        return DRACONIC_HOST_E_INVAL;
    }
    end = byte_offset + byte_length;
    if (end < byte_offset || end > parent->len) {
        return DRACONIC_HOST_E_INVAL;
    }

    if (byte_length == 0) {
        /* Empty view: data may be NULL or one-past when offset == parent->len. */
        out->data = (parent->data && byte_offset < parent->len)
            ? parent->data + byte_offset
            : NULL;
        out->len = 0;
        return DRACONIC_HOST_OK;
    }

    out->data = parent->data + byte_offset;
    out->len = byte_length;
    return DRACONIC_HOST_OK;
}

DraconicHostError draconic_rt_host_bytes_alloc(
    size_t len,
    uint8_t **out_data) {
    uint8_t *buf;

    if (!out_data) {
        return DRACONIC_HOST_E_INVAL;
    }
    *out_data = NULL;

    if (len == 0) {
        return DRACONIC_HOST_OK;
    }

    buf = (uint8_t *)calloc(len, 1);
    if (!buf) {
        return DRACONIC_HOST_E_NOMEM;
    }
    *out_data = buf;
    return DRACONIC_HOST_OK;
}

void draconic_rt_host_bytes_storage_free(uint8_t *data) {
    free(data);
}

DraconicHostError draconic_rt_host_bytes_copy_in(
    DraconicHostBytes *dst,
    const uint8_t *src,
    size_t src_len,
    size_t *out_n) {
    size_t n;

    if (!dst || !out_n) {
        return DRACONIC_HOST_E_INVAL;
    }
    *out_n = 0;

    if (dst->len > 0 && !dst->data) {
        return DRACONIC_HOST_E_INVAL;
    }
    if (src_len > 0 && !src) {
        return DRACONIC_HOST_E_INVAL;
    }

    n = dst->len < src_len ? dst->len : src_len;
    if (n > 0) {
        memcpy(dst->data, src, n);
    }
    *out_n = n;
    return DRACONIC_HOST_OK;
}

DraconicHostError draconic_rt_host_bytes_copy_out(
    const DraconicHostBytes *src,
    uint8_t *dst,
    size_t dst_cap,
    size_t *out_n) {
    size_t n;

    if (!src || !out_n) {
        return DRACONIC_HOST_E_INVAL;
    }
    *out_n = 0;

    if (src->len > 0 && !src->data) {
        return DRACONIC_HOST_E_INVAL;
    }
    if (dst_cap > 0 && !dst) {
        return DRACONIC_HOST_E_INVAL;
    }

    n = src->len < dst_cap ? src->len : dst_cap;
    if (n > 0) {
        memcpy(dst, src->data, n);
    }
    *out_n = n;
    return DRACONIC_HOST_OK;
}

/* --- Process argv (H01.01) --- */

static int g_process_argc;
static char **g_process_argv;

void draconic_rt_host_process_set_argv(int argc, char **argv) {
    if (argc < 0) {
        argc = 0;
    }
    g_process_argc = argc;
    g_process_argv = argv;
}

int32_t draconic_rt_host_process_user_argc(void) {
    if (g_process_argc <= 1) {
        return 0;
    }
    return (int32_t)(g_process_argc - 1);
}

const char *draconic_rt_host_process_user_arg(int32_t i) {
    if (i < 0 || g_process_argc <= 1) {
        return NULL;
    }
    if (i >= g_process_argc - 1) {
        return NULL;
    }
    if (!g_process_argv) {
        return NULL;
    }
    return g_process_argv[i + 1];
}

/* --- Process env (H01.02) --- */

char *draconic_rt_host_env_get(const char *key) {
    const char *v;
    size_t n;
    char *out;

    if (!key) {
        return NULL;
    }
    v = getenv(key);
    if (!v) {
        return NULL;
    }
    n = strlen(v);
    out = (char *)malloc(n + 1);
    if (!out) {
        return NULL;
    }
    memcpy(out, v, n + 1);
    return out;
}

int32_t draconic_rt_host_env_set(const char *key, const char *value) {
    if (!key || !value) {
        return -1;
    }
#if defined(_WIN32)
    return _putenv_s(key, value) == 0 ? 0 : -1;
#else
    return setenv(key, value, 1) == 0 ? 0 : -1;
#endif
}

int32_t draconic_rt_host_env_delete(const char *key) {
    if (!key) {
        return -1;
    }
#if defined(_WIN32)
    /* Empty value removes the variable with _putenv_s on MSVC. */
    return _putenv_s(key, "") == 0 ? 0 : -1;
#else
    return unsetenv(key) == 0 ? 0 : -1;
#endif
}

/* --- Process exit (H01.03) --- */

static int32_t g_process_exit_code;

void draconic_rt_host_process_exit(int32_t code) {
    g_process_exit_code = code;
    exit((int)code);
}

void draconic_rt_host_process_set_exit_code(int32_t code) {
    g_process_exit_code = code;
}

int32_t draconic_rt_host_process_get_exit_code(void) {
    return g_process_exit_code;
}

/* --- Process pid / ppid (H01.04) --- */

int32_t draconic_rt_host_process_pid(void) {
#if defined(_WIN32)
    return (int32_t)GetCurrentProcessId();
#else
    return (int32_t)getpid();
#endif
}

int32_t draconic_rt_host_process_ppid(void) {
#if defined(_WIN32)
    {
        DWORD self = GetCurrentProcessId();
        DWORD parent = 0;
        HANDLE snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        PROCESSENTRY32 pe;
        if (snap == INVALID_HANDLE_VALUE) {
            return 0;
        }
        pe.dwSize = sizeof(pe);
        if (Process32First(snap, &pe)) {
            do {
                if (pe.th32ProcessID == self) {
                    parent = pe.th32ParentProcessID;
                    break;
                }
            } while (Process32Next(snap, &pe));
        }
        CloseHandle(snap);
        return (int32_t)parent;
    }
#else
    return (int32_t)getppid();
#endif
}

/* --- Wall clock (H05.01) --- */

double draconic_rt_host_now_ms(void) {
#if defined(_WIN32)
    {
        /* FILETIME is 100-ns intervals since 1601-01-01 UTC. */
        FILETIME ft;
        ULARGE_INTEGER u;
        const uint64_t epoch_diff_100ns = 116444736000000000ULL;
        GetSystemTimeAsFileTime(&ft);
        u.LowPart = ft.dwLowDateTime;
        u.HighPart = ft.dwHighDateTime;
        if (u.QuadPart < epoch_diff_100ns) {
            return 0.0;
        }
        return (double)((u.QuadPart - epoch_diff_100ns) / 10000ULL);
    }
#else
    {
        struct timeval tv;
        if (gettimeofday(&tv, NULL) != 0) {
            return 0.0;
        }
        return ((double)tv.tv_sec * 1000.0) + ((double)tv.tv_usec / 1000.0);
    }
#endif
}

/* --- Monotonic clock (H05.02) --- */

double draconic_rt_host_monotonic_ms(void) {
#if defined(_WIN32)
    {
        static LARGE_INTEGER freq;
        static int have_freq = 0;
        LARGE_INTEGER counter;
        if (!have_freq) {
            if (!QueryPerformanceFrequency(&freq) || freq.QuadPart == 0) {
                return 0.0;
            }
            have_freq = 1;
        }
        if (!QueryPerformanceCounter(&counter)) {
            return 0.0;
        }
        return ((double)counter.QuadPart * 1000.0) / (double)freq.QuadPart;
    }
#else
    {
        struct timespec ts;
        if (clock_gettime(CLOCK_MONOTONIC, &ts) != 0) {
            return 0.0;
        }
        return ((double)ts.tv_sec * 1000.0) + ((double)ts.tv_nsec / 1000000.0);
    }
#endif
}

/* --- Stdout write (H02.01) --- */

DraconicHostError draconic_rt_host_stdout_write(const uint8_t *data, size_t len) {
    size_t n;
    if (len == 0) {
        return DRACONIC_HOST_OK;
    }
    if (!data) {
        return DRACONIC_HOST_E_INVAL;
    }
    n = fwrite(data, 1, len, stdout);
    if (n != len) {
        return DRACONIC_HOST_E_IO;
    }
    if (fflush(stdout) != 0) {
        return DRACONIC_HOST_E_IO;
    }
    return DRACONIC_HOST_OK;
}

/* --- Stderr write (H02.02) --- */

DraconicHostError draconic_rt_host_stderr_write(const uint8_t *data, size_t len) {
    size_t n;
    if (len == 0) {
        return DRACONIC_HOST_OK;
    }
    if (!data) {
        return DRACONIC_HOST_E_INVAL;
    }
    n = fwrite(data, 1, len, stderr);
    if (n != len) {
        return DRACONIC_HOST_E_IO;
    }
    if (fflush(stderr) != 0) {
        return DRACONIC_HOST_E_IO;
    }
    return DRACONIC_HOST_OK;
}

/* --- Stdin read (H02.03) --- */

char *draconic_rt_host_stdin_read_line(void) {
    size_t cap = 64;
    size_t len = 0;
    char *buf = (char *)malloc(cap);
    int ch;

    if (!buf) {
        return NULL;
    }

    for (;;) {
        ch = fgetc(stdin);
        if (ch == EOF) {
            if (len == 0) {
                free(buf);
                return NULL;
            }
            break;
        }
        if (ch == '\n') {
            break;
        }
        if (len + 1 >= cap) {
            size_t ncap = cap * 2;
            char *nbuf = (char *)realloc(buf, ncap);
            if (!nbuf) {
                free(buf);
                return NULL;
            }
            buf = nbuf;
            cap = ncap;
        }
        buf[len++] = (char)ch;
    }

    /* Strip trailing CR from CRLF. */
    if (len > 0 && buf[len - 1] == '\r') {
        len--;
    }
    buf[len] = '\0';
    return buf;
}

DraconicHostError draconic_rt_host_stdin_read_bytes(
    size_t max_len,
    uint8_t **out_data,
    size_t *out_len) {
    uint8_t *buf;
    size_t n;

    if (!out_data || !out_len) {
        return DRACONIC_HOST_E_INVAL;
    }
    *out_data = NULL;
    *out_len = 0;

    if (max_len == 0) {
        return DRACONIC_HOST_OK;
    }

    buf = (uint8_t *)malloc(max_len);
    if (!buf) {
        return DRACONIC_HOST_E_NOMEM;
    }

    n = fread(buf, 1, max_len, stdin);
    if (n == 0) {
        free(buf);
        if (ferror(stdin)) {
            return DRACONIC_HOST_E_IO;
        }
        return DRACONIC_HOST_OK;
    }

    if (n < max_len) {
        uint8_t *shrunk = (uint8_t *)realloc(buf, n);
        if (shrunk) {
            buf = shrunk;
        }
    }
    *out_data = buf;
    *out_len = n;
    return DRACONIC_HOST_OK;
}

/* --- Path helpers (H03.01–H03.02): join/normalize/dirname/basename/extname/isAbsolute --- */

static int host_path_is_sep(char c) {
    return c == '/' || c == '\\';
}

/* Node path.posix-style normalize; accepts `\` as separator too. */
char *draconic_rt_host_path_normalize(const char *path) {
    const char *src;
    size_t len;
    size_t i;
    int is_abs;
    int trailing_sep;
    char *out;
    size_t stack_cap;
    size_t *starts = NULL;
    size_t *ends = NULL;
    size_t nseg = 0;
    size_t need;
    size_t pos;
    size_t s;

    src = path ? path : "";
    len = strlen(src);
    if (len == 0) {
        out = (char *)malloc(2);
        if (!out) {
            return NULL;
        }
        out[0] = '.';
        out[1] = '\0';
        return out;
    }

    is_abs = host_path_is_sep(src[0]);
    trailing_sep = host_path_is_sep(src[len - 1]);

    stack_cap = len / 1 + 4;
    starts = (size_t *)malloc(stack_cap * sizeof(size_t));
    ends = (size_t *)malloc(stack_cap * sizeof(size_t));
    if (!starts || !ends) {
        free(starts);
        free(ends);
        return NULL;
    }

    i = 0;
    while (i < len) {
        size_t seg_start;
        size_t seg_end;
        size_t seglen;

        while (i < len && host_path_is_sep(src[i])) {
            i++;
        }
        if (i >= len) {
            break;
        }
        seg_start = i;
        while (i < len && !host_path_is_sep(src[i])) {
            i++;
        }
        seg_end = i;
        seglen = seg_end - seg_start;

        if (seglen == 1 && src[seg_start] == '.') {
            continue;
        }
        if (seglen == 2 && src[seg_start] == '.' && src[seg_start + 1] == '.') {
            if (nseg > 0) {
                size_t ps = starts[nseg - 1];
                size_t pe = ends[nseg - 1];
                if (!(pe - ps == 2 && src[ps] == '.' && src[ps + 1] == '.')) {
                    nseg--;
                    continue;
                }
            }
            if (!is_abs) {
                if (nseg >= stack_cap) {
                    size_t nc = stack_cap * 2;
                    size_t *ns = (size_t *)realloc(starts, nc * sizeof(size_t));
                    size_t *ne = (size_t *)realloc(ends, nc * sizeof(size_t));
                    if (!ns || !ne) {
                        free(ns ? ns : starts);
                        free(ne ? ne : ends);
                        return NULL;
                    }
                    starts = ns;
                    ends = ne;
                    stack_cap = nc;
                }
                starts[nseg] = seg_start;
                ends[nseg] = seg_end;
                nseg++;
            }
            continue;
        }

        if (nseg >= stack_cap) {
            size_t nc = stack_cap * 2;
            size_t *ns = (size_t *)realloc(starts, nc * sizeof(size_t));
            size_t *ne = (size_t *)realloc(ends, nc * sizeof(size_t));
            if (!ns || !ne) {
                free(ns ? ns : starts);
                free(ne ? ne : ends);
                return NULL;
            }
            starts = ns;
            ends = ne;
            stack_cap = nc;
        }
        starts[nseg] = seg_start;
        ends[nseg] = seg_end;
        nseg++;
    }

    need = 1; /* NUL */
    if (is_abs) {
        need += 1;
    }
    if (nseg == 0) {
        if (!is_abs) {
            need += 1; /* '.' */
        }
    } else {
        for (s = 0; s < nseg; s++) {
            need += ends[s] - starts[s];
            if (s + 1 < nseg) {
                need += 1;
            }
        }
        if (trailing_sep) {
            need += 1;
        }
    }

    out = (char *)malloc(need);
    if (!out) {
        free(starts);
        free(ends);
        return NULL;
    }
    pos = 0;
    if (is_abs) {
        out[pos++] = '/';
    }
    if (nseg == 0) {
        if (!is_abs) {
            out[pos++] = '.';
        }
    } else {
        for (s = 0; s < nseg; s++) {
            size_t seglen = ends[s] - starts[s];
            memcpy(out + pos, src + starts[s], seglen);
            pos += seglen;
            if (s + 1 < nseg) {
                out[pos++] = '/';
            }
        }
        if (trailing_sep) {
            out[pos++] = '/';
        }
    }
    out[pos] = '\0';
    free(starts);
    free(ends);
    return out;
}

char *draconic_rt_host_path_join(size_t n, const char *const *parts) {
    size_t total = 0;
    size_t i;
    size_t used = 0;
    char *joined;
    char *norm;
    size_t jlen;

    if (n == 0 || !parts) {
        return draconic_rt_host_path_normalize("");
    }

    for (i = 0; i < n; i++) {
        const char *p = parts[i] ? parts[i] : "";
        size_t pl = strlen(p);
        if (pl == 0) {
            continue;
        }
        if (used > 0) {
            total += 1; /* '/' */
        }
        total += pl;
        used++;
    }
    if (used == 0) {
        return draconic_rt_host_path_normalize("");
    }

    joined = (char *)malloc(total + 1);
    if (!joined) {
        return NULL;
    }
    jlen = 0;
    used = 0;
    for (i = 0; i < n; i++) {
        const char *p = parts[i] ? parts[i] : "";
        size_t pl = strlen(p);
        if (pl == 0) {
            continue;
        }
        if (used > 0) {
            joined[jlen++] = '/';
        }
        memcpy(joined + jlen, p, pl);
        jlen += pl;
        used++;
    }
    joined[jlen] = '\0';
    norm = draconic_rt_host_path_normalize(joined);
    free(joined);
    return norm;
}

/* Copy src[0..len) into malloc'd string, mapping `\` → `/`. */
static char *host_path_dup_norm_seps(const char *src, size_t len) {
    char *out;
    size_t i;
    out = (char *)malloc(len + 1);
    if (!out) {
        return NULL;
    }
    for (i = 0; i < len; i++) {
        char c = src[i];
        out[i] = (c == '\\') ? '/' : c;
    }
    out[len] = '\0';
    return out;
}

static char *host_path_strdup_lit(const char *s) {
    size_t n = strlen(s);
    char *out = (char *)malloc(n + 1);
    if (!out) {
        return NULL;
    }
    memcpy(out, s, n + 1);
    return out;
}

char *draconic_rt_host_path_dirname(const char *path) {
    const char *src = path ? path : "";
    size_t len = strlen(src);
    size_t end;
    size_t i;
    size_t dend;

    if (len == 0) {
        return host_path_strdup_lit(".");
    }

    end = len;
    while (end > 0 && host_path_is_sep(src[end - 1])) {
        end--;
    }
    if (end == 0) {
        return host_path_strdup_lit("/");
    }

    i = end;
    while (i > 0 && !host_path_is_sep(src[i - 1])) {
        i--;
    }
    if (i == 0) {
        return host_path_strdup_lit(".");
    }

    dend = i;
    while (dend > 0 && host_path_is_sep(src[dend - 1])) {
        dend--;
    }
    if (dend == 0) {
        return host_path_strdup_lit("/");
    }
    return host_path_dup_norm_seps(src, dend);
}

char *draconic_rt_host_path_basename(const char *path) {
    const char *src = path ? path : "";
    size_t len = strlen(src);
    size_t end;
    size_t i;

    if (len == 0) {
        return host_path_strdup_lit("");
    }

    end = len;
    while (end > 0 && host_path_is_sep(src[end - 1])) {
        end--;
    }
    if (end == 0) {
        return host_path_strdup_lit("");
    }

    i = end;
    while (i > 0 && !host_path_is_sep(src[i - 1])) {
        i--;
    }
    return host_path_dup_norm_seps(src + i, end - i);
}

char *draconic_rt_host_path_extname(const char *path) {
    const char *src = path ? path : "";
    size_t len = strlen(src);
    size_t i;
    size_t end = 0;
    size_t start_dot = (size_t)-1;
    size_t start_part = 0;
    int matched_slash = 1;
    int pre_dot_state = 0;
    int has_end = 0;
    int has_start_dot = 0;

    if (len == 0) {
        return host_path_strdup_lit("");
    }

    for (i = len; i > 0; i--) {
        char c = src[i - 1];
        if (host_path_is_sep(c)) {
            if (!matched_slash) {
                start_part = i;
                break;
            }
            continue;
        }
        if (!has_end) {
            matched_slash = 0;
            end = i;
            has_end = 1;
        }
        if (c == '.') {
            if (!has_start_dot) {
                start_dot = i - 1;
                has_start_dot = 1;
            } else if (pre_dot_state != 1) {
                pre_dot_state = 1;
            }
        } else if (has_start_dot) {
            pre_dot_state = -1;
        }
    }

    if (!has_start_dot || !has_end || pre_dot_state == 0 ||
        (pre_dot_state == 1 && start_dot == end - 1 && start_dot == start_part + 1)) {
        return host_path_strdup_lit("");
    }
    return host_path_dup_norm_seps(src + start_dot, end - start_dot);
}

int32_t draconic_rt_host_path_is_absolute(const char *path) {
    if (!path || path[0] == '\0') {
        return 0;
    }
    return host_path_is_sep(path[0]) ? 1 : 0;
}

/* --- Filesystem read (H04.01) -------------------------------------------- */

DraconicHostError draconic_rt_host_fs_read_file(
    const char *path,
    uint8_t **out_data,
    size_t *out_len) {
    FILE *f;
    long sz;
    size_t n;
    uint8_t *buf;

    if (!path || path[0] == '\0' || !out_data || !out_len) {
        return DRACONIC_HOST_E_INVAL;
    }
    *out_data = NULL;
    *out_len = 0;

    f = fopen(path, "rb");
    if (!f) {
        if (errno == ENOENT || errno == ENOTDIR) {
            return DRACONIC_HOST_E_NOENT;
        }
        if (errno == EACCES
#if defined(EPERM)
            || errno == EPERM
#endif
        ) {
            return DRACONIC_HOST_E_PERM;
        }
        return DRACONIC_HOST_E_IO;
    }

    if (fseek(f, 0, SEEK_END) != 0) {
        fclose(f);
        return DRACONIC_HOST_E_IO;
    }
    sz = ftell(f);
    if (sz < 0) {
        fclose(f);
        return DRACONIC_HOST_E_IO;
    }
    if (fseek(f, 0, SEEK_SET) != 0) {
        fclose(f);
        return DRACONIC_HOST_E_IO;
    }

    n = (size_t)sz;
    if (n == 0) {
        fclose(f);
        return DRACONIC_HOST_OK;
    }

    buf = (uint8_t *)malloc(n);
    if (!buf) {
        fclose(f);
        return DRACONIC_HOST_E_NOMEM;
    }
    if (fread(buf, 1, n, f) != n) {
        free(buf);
        fclose(f);
        return DRACONIC_HOST_E_IO;
    }
    fclose(f);
    *out_data = buf;
    *out_len = n;
    return DRACONIC_HOST_OK;
}

DraconicHostError draconic_rt_host_fs_read_text(
    const char *path,
    char **out_text) {
    uint8_t *data = NULL;
    size_t len = 0;
    DraconicHostError err;
    char *text;

    if (!out_text) {
        return DRACONIC_HOST_E_INVAL;
    }
    *out_text = NULL;

    err = draconic_rt_host_fs_read_file(path, &data, &len);
    if (err != DRACONIC_HOST_OK) {
        return err;
    }

    if (len > 0 && !host_utf8_is_valid(data, len)) {
        free(data);
        return DRACONIC_HOST_E_INVAL;
    }

    text = (char *)malloc(len + 1);
    if (!text) {
        free(data);
        return DRACONIC_HOST_E_NOMEM;
    }
    if (len > 0) {
        memcpy(text, data, len);
    }
    text[len] = '\0';
    free(data);
    *out_text = text;
    return DRACONIC_HOST_OK;
}

/* --- Filesystem write / append (H04.02) ---------------------------------- */

static DraconicHostError host_fs_write_mode(
    const char *path,
    const uint8_t *data,
    size_t len,
    const char *mode) {
    FILE *f;
    size_t n;

    if (!path || path[0] == '\0') {
        return DRACONIC_HOST_E_INVAL;
    }
    if (len > 0 && !data) {
        return DRACONIC_HOST_E_INVAL;
    }

    f = fopen(path, mode);
    if (!f) {
        if (errno == ENOENT || errno == ENOTDIR) {
            return DRACONIC_HOST_E_NOENT;
        }
        if (errno == EACCES
#if defined(EPERM)
            || errno == EPERM
#endif
        ) {
            return DRACONIC_HOST_E_PERM;
        }
        return DRACONIC_HOST_E_IO;
    }

    if (len == 0) {
        fclose(f);
        return DRACONIC_HOST_OK;
    }

    n = fwrite(data, 1, len, f);
    if (n != len) {
        fclose(f);
        return DRACONIC_HOST_E_IO;
    }
    if (fclose(f) != 0) {
        return DRACONIC_HOST_E_IO;
    }
    return DRACONIC_HOST_OK;
}

DraconicHostError draconic_rt_host_fs_write_file(
    const char *path,
    const uint8_t *data,
    size_t len) {
    return host_fs_write_mode(path, data, len, "wb");
}

DraconicHostError draconic_rt_host_fs_append_file(
    const char *path,
    const uint8_t *data,
    size_t len) {
    return host_fs_write_mode(path, data, len, "ab");
}

DraconicHostError draconic_rt_host_fs_write_text(
    const char *path,
    const char *text) {
    const char *t = text ? text : "";
    size_t len = strlen(t);
    return draconic_rt_host_fs_write_file(path, (const uint8_t *)t, len);
}

DraconicHostError draconic_rt_host_fs_append_text(
    const char *path,
    const char *text) {
    const char *t = text ? text : "";
    size_t len = strlen(t);
    return draconic_rt_host_fs_append_file(path, (const uint8_t *)t, len);
}

/* --- Filesystem exists / stat (H04.03) ----------------------------------- */

int32_t draconic_rt_host_fs_exists(const char *path) {
    struct stat st;
    if (!path || path[0] == '\0') {
        return 0;
    }
    if (stat(path, &st) != 0) {
        return 0;
    }
    return 1;
}

DraconicHostError draconic_rt_host_fs_stat(
    const char *path,
    int64_t *out_size,
    int32_t *out_is_file,
    int32_t *out_is_dir,
    double *out_mtime_ms) {
    struct stat st;
    double ms;

    if (!path || path[0] == '\0' || !out_size || !out_is_file || !out_is_dir
        || !out_mtime_ms) {
        return DRACONIC_HOST_E_INVAL;
    }
    *out_size = 0;
    *out_is_file = 0;
    *out_is_dir = 0;
    *out_mtime_ms = 0.0;

    if (stat(path, &st) != 0) {
        if (errno == ENOENT || errno == ENOTDIR) {
            return DRACONIC_HOST_E_NOENT;
        }
        if (errno == EACCES
#if defined(EPERM)
            || errno == EPERM
#endif
        ) {
            return DRACONIC_HOST_E_PERM;
        }
        return DRACONIC_HOST_E_IO;
    }

    *out_size = (int64_t)st.st_size;
    *out_is_file = S_ISREG(st.st_mode) ? 1 : 0;
    *out_is_dir = S_ISDIR(st.st_mode) ? 1 : 0;

#if defined(__APPLE__)
    ms = (double)st.st_mtimespec.tv_sec * 1000.0
        + (double)st.st_mtimespec.tv_nsec / 1000000.0;
#elif defined(_WIN32)
    ms = (double)st.st_mtime * 1000.0;
#else
    ms = (double)st.st_mtim.tv_sec * 1000.0
        + (double)st.st_mtim.tv_nsec / 1000000.0;
#endif
    *out_mtime_ms = ms;
    return DRACONIC_HOST_OK;
}

/* --- Filesystem directory ops (H04.04) ----------------------------------- */

static DraconicHostError host_fs_errno_map(void) {
    if (errno == ENOENT || errno == ENOTDIR) {
        return DRACONIC_HOST_E_NOENT;
    }
    if (errno == EEXIST) {
        return DRACONIC_HOST_E_EXIST;
    }
    if (errno == EACCES
#if defined(EPERM)
        || errno == EPERM
#endif
    ) {
        return DRACONIC_HOST_E_PERM;
    }
    if (errno == ENOMEM) {
        return DRACONIC_HOST_E_NOMEM;
    }
    return DRACONIC_HOST_E_IO;
}

static DraconicHostError host_fs_mkdir_one(const char *path) {
#if defined(_WIN32)
    if (_mkdir(path) == 0) {
        return DRACONIC_HOST_OK;
    }
#else
    if (mkdir(path, 0755) == 0) {
        return DRACONIC_HOST_OK;
    }
#endif
    return host_fs_errno_map();
}

DraconicHostError draconic_rt_host_fs_mkdir(const char *path) {
    if (!path || path[0] == '\0') {
        return DRACONIC_HOST_E_INVAL;
    }
    return host_fs_mkdir_one(path);
}

DraconicHostError draconic_rt_host_fs_mkdir_all(const char *path) {
    char *buf;
    size_t len;
    size_t i;
    DraconicHostError err;
    struct stat st;

    if (!path || path[0] == '\0') {
        return DRACONIC_HOST_E_INVAL;
    }

    /* Already a directory → OK (mkdir -p). */
    if (stat(path, &st) == 0) {
        if (S_ISDIR(st.st_mode)) {
            return DRACONIC_HOST_OK;
        }
        return DRACONIC_HOST_E_EXIST;
    }

    len = strlen(path);
    buf = (char *)malloc(len + 1);
    if (!buf) {
        return DRACONIC_HOST_E_NOMEM;
    }
    memcpy(buf, path, len + 1);

    /* Walk components; create each prefix. Skip drive letter on Windows. */
    i = 0;
#if defined(_WIN32)
    if (len >= 2 && ((buf[0] >= 'A' && buf[0] <= 'Z') || (buf[0] >= 'a' && buf[0] <= 'z'))
        && buf[1] == ':') {
        i = 2;
    }
#endif
    if (buf[i] == '/' || buf[i] == '\\') {
        i++;
    }
    for (; i < len; i++) {
        if (buf[i] == '/' || buf[i] == '\\') {
            char save = buf[i];
            buf[i] = '\0';
            if (buf[0] != '\0' && !(buf[0] == '/' && buf[1] == '\0')) {
                err = host_fs_mkdir_one(buf);
                if (err != DRACONIC_HOST_OK && err != DRACONIC_HOST_E_EXIST) {
                    free(buf);
                    return err;
                }
            }
            buf[i] = save;
        }
    }
    err = host_fs_mkdir_one(buf);
    free(buf);
    if (err == DRACONIC_HOST_E_EXIST) {
        if (stat(path, &st) == 0 && S_ISDIR(st.st_mode)) {
            return DRACONIC_HOST_OK;
        }
    }
    return err;
}

DraconicHostError draconic_rt_host_fs_readdir(
    const char *path,
    char ***out_names,
    int64_t *out_count) {
    char **names = NULL;
    size_t count = 0;
    size_t cap = 0;

    if (!path || path[0] == '\0' || !out_names || !out_count) {
        return DRACONIC_HOST_E_INVAL;
    }
    *out_names = NULL;
    *out_count = 0;

#if defined(_WIN32)
    {
        char pattern[MAX_PATH];
        WIN32_FIND_DATAA fd;
        HANDLE h;
        size_t plen = strlen(path);
        if (plen + 3 >= sizeof(pattern)) {
            return DRACONIC_HOST_E_INVAL;
        }
        memcpy(pattern, path, plen);
        if (plen > 0 && path[plen - 1] != '/' && path[plen - 1] != '\\') {
            pattern[plen++] = '\\';
        }
        pattern[plen++] = '*';
        pattern[plen] = '\0';
        h = FindFirstFileA(pattern, &fd);
        if (h == INVALID_HANDLE_VALUE) {
            DWORD e = GetLastError();
            if (e == ERROR_FILE_NOT_FOUND || e == ERROR_PATH_NOT_FOUND) {
                return DRACONIC_HOST_E_NOENT;
            }
            return DRACONIC_HOST_E_IO;
        }
        do {
            const char *name = fd.cFileName;
            char *copy;
            char **grown;
            if (strcmp(name, ".") == 0 || strcmp(name, "..") == 0) {
                continue;
            }
            if (count == cap) {
                size_t ncap = cap == 0 ? 8 : cap * 2;
                grown = (char **)realloc(names, ncap * sizeof(char *));
                if (!grown) {
                    FindClose(h);
                    while (count > 0) {
                        free(names[--count]);
                    }
                    free(names);
                    return DRACONIC_HOST_E_NOMEM;
                }
                names = grown;
                cap = ncap;
            }
            copy = (char *)malloc(strlen(name) + 1);
            if (!copy) {
                FindClose(h);
                while (count > 0) {
                    free(names[--count]);
                }
                free(names);
                return DRACONIC_HOST_E_NOMEM;
            }
            memcpy(copy, name, strlen(name) + 1);
            names[count++] = copy;
        } while (FindNextFileA(h, &fd));
        FindClose(h);
    }
#else
    {
        DIR *dir = opendir(path);
        struct dirent *ent;
        if (!dir) {
            return host_fs_errno_map();
        }
        while ((ent = readdir(dir)) != NULL) {
            const char *name = ent->d_name;
            char *copy;
            char **grown;
            if (strcmp(name, ".") == 0 || strcmp(name, "..") == 0) {
                continue;
            }
            if (count == cap) {
                size_t ncap = cap == 0 ? 8 : cap * 2;
                grown = (char **)realloc(names, ncap * sizeof(char *));
                if (!grown) {
                    closedir(dir);
                    while (count > 0) {
                        free(names[--count]);
                    }
                    free(names);
                    return DRACONIC_HOST_E_NOMEM;
                }
                names = grown;
                cap = ncap;
            }
            copy = (char *)malloc(strlen(name) + 1);
            if (!copy) {
                closedir(dir);
                while (count > 0) {
                    free(names[--count]);
                }
                free(names);
                return DRACONIC_HOST_E_NOMEM;
            }
            memcpy(copy, name, strlen(name) + 1);
            names[count++] = copy;
        }
        closedir(dir);
    }
#endif

    *out_names = names;
    *out_count = (int64_t)count;
    return DRACONIC_HOST_OK;
}

DraconicHostError draconic_rt_host_fs_rmdir(const char *path) {
    if (!path || path[0] == '\0') {
        return DRACONIC_HOST_E_INVAL;
    }
#if defined(_WIN32)
    if (_rmdir(path) == 0) {
        return DRACONIC_HOST_OK;
    }
#else
    if (rmdir(path) == 0) {
        return DRACONIC_HOST_OK;
    }
#endif
    return host_fs_errno_map();
}

DraconicHostError draconic_rt_host_fs_remove_file(const char *path) {
    if (!path || path[0] == '\0') {
        return DRACONIC_HOST_E_INVAL;
    }
#if defined(_WIN32)
    if (_unlink(path) == 0) {
        return DRACONIC_HOST_OK;
    }
#else
    if (unlink(path) == 0) {
        return DRACONIC_HOST_OK;
    }
#endif
    return host_fs_errno_map();
}

DraconicHostError draconic_rt_host_fs_rename_file(const char *from, const char *to) {
    if (!from || from[0] == '\0' || !to || to[0] == '\0') {
        return DRACONIC_HOST_E_INVAL;
    }
#if defined(_WIN32)
    if (MoveFileExA(from, to, MOVEFILE_REPLACE_EXISTING | MOVEFILE_COPY_ALLOWED) != 0) {
        return DRACONIC_HOST_OK;
    }
    /* Fall back to CRT rename when MoveFileEx unavailable path. */
    if (rename(from, to) == 0) {
        return DRACONIC_HOST_OK;
    }
#else
    if (rename(from, to) == 0) {
        return DRACONIC_HOST_OK;
    }
#endif
    return host_fs_errno_map();
}

DraconicHostError draconic_rt_host_fs_copy_file(const char *from, const char *to) {
    uint8_t *data = NULL;
    size_t len = 0;
    DraconicHostError err;

    if (!from || from[0] == '\0' || !to || to[0] == '\0') {
        return DRACONIC_HOST_E_INVAL;
    }
    err = draconic_rt_host_fs_read_file(from, &data, &len);
    if (err != DRACONIC_HOST_OK) {
        return err;
    }
    err = draconic_rt_host_fs_write_file(to, data, len);
    free(data);
    return err;
}

/* --- Open handle: open / read / write / seek / close (H04.06) -------------- */

static int host_fs_parse_open_mode(const char *mode, int *out_flags) {
    int flags = 0;
    if (!mode || !out_flags) {
        return 0;
    }
    if (strcmp(mode, "r") == 0) {
        flags = O_RDONLY;
    } else if (strcmp(mode, "w") == 0) {
        flags = O_WRONLY | O_CREAT | O_TRUNC;
    } else if (strcmp(mode, "a") == 0) {
        flags = O_WRONLY | O_CREAT | O_APPEND;
    } else if (strcmp(mode, "r+") == 0) {
        flags = O_RDWR;
    } else if (strcmp(mode, "w+") == 0) {
        flags = O_RDWR | O_CREAT | O_TRUNC;
    } else if (strcmp(mode, "a+") == 0) {
        flags = O_RDWR | O_CREAT | O_APPEND;
    } else {
        return 0;
    }
#if defined(_WIN32)
    flags |= O_BINARY;
#endif
    *out_flags = flags;
    return 1;
}

DraconicHostError draconic_rt_host_fs_open(
    const char *path,
    const char *mode,
    DraconicHostHandle *out_h) {
    int flags = 0;
    int fd;
    DraconicHostError err;

    if (!path || path[0] == '\0' || !mode || !out_h) {
        return DRACONIC_HOST_E_INVAL;
    }
    if (!host_fs_parse_open_mode(mode, &flags)) {
        return DRACONIC_HOST_E_INVAL;
    }
#if defined(_WIN32)
    fd = _open(path, flags, _S_IREAD | _S_IWRITE);
#else
    fd = open(path, flags, 0666);
#endif
    if (fd < 0) {
        return host_fs_errno_map();
    }
    err = host_handle_alloc(DRACONIC_HOST_HANDLE_KIND_FILE, fd, out_h);
    if (err != DRACONIC_HOST_OK) {
#if defined(_WIN32)
        (void)_close(fd);
#else
        (void)close(fd);
#endif
        return err;
    }
    return DRACONIC_HOST_OK;
}

DraconicHostError draconic_rt_host_fs_handle_read(
    DraconicHostHandle h,
    size_t max_len,
    uint8_t **out_data,
    size_t *out_len) {
    int fd;
    uint8_t *buf = NULL;
    size_t got = 0;

    if (!out_data || !out_len) {
        return DRACONIC_HOST_E_INVAL;
    }
    *out_data = NULL;
    *out_len = 0;
    fd = host_handle_fd(h);
    if (fd < 0) {
        return DRACONIC_HOST_E_BADF;
    }
    if (max_len == 0) {
        return DRACONIC_HOST_OK;
    }
    buf = (uint8_t *)malloc(max_len);
    if (!buf) {
        return DRACONIC_HOST_E_NOMEM;
    }
#if defined(_WIN32)
    {
        int n = _read(fd, buf, (unsigned int)(max_len > 0x7fffffffu ? 0x7fffffffu : max_len));
        if (n < 0) {
            free(buf);
            return host_fs_errno_map();
        }
        got = (size_t)n;
    }
#else
    {
        ssize_t n = read(fd, buf, max_len);
        if (n < 0) {
            free(buf);
            return host_fs_errno_map();
        }
        got = (size_t)n;
    }
#endif
    if (got == 0) {
        free(buf);
        *out_data = NULL;
        *out_len = 0;
        return DRACONIC_HOST_OK;
    }
    if (got < max_len) {
        uint8_t *shrunk = (uint8_t *)realloc(buf, got);
        if (shrunk) {
            buf = shrunk;
        }
    }
    *out_data = buf;
    *out_len = got;
    return DRACONIC_HOST_OK;
}

DraconicHostError draconic_rt_host_fs_handle_write(
    DraconicHostHandle h,
    const uint8_t *data,
    size_t len) {
    int fd;
    size_t off = 0;

    fd = host_handle_fd(h);
    if (fd < 0) {
        return DRACONIC_HOST_E_BADF;
    }
    if (len == 0) {
        return DRACONIC_HOST_OK;
    }
    if (!data) {
        return DRACONIC_HOST_E_INVAL;
    }
    while (off < len) {
#if defined(_WIN32)
        int n = _write(
            fd,
            data + off,
            (unsigned int)((len - off) > 0x7fffffffu ? 0x7fffffffu : (len - off)));
        if (n < 0) {
            return host_fs_errno_map();
        }
        if (n == 0) {
            return DRACONIC_HOST_E_IO;
        }
        off += (size_t)n;
#else
        ssize_t n = write(fd, data + off, len - off);
        if (n < 0) {
            return host_fs_errno_map();
        }
        if (n == 0) {
            return DRACONIC_HOST_E_IO;
        }
        off += (size_t)n;
#endif
    }
    return DRACONIC_HOST_OK;
}

DraconicHostError draconic_rt_host_fs_handle_seek(
    DraconicHostHandle h,
    int64_t offset,
    int32_t whence,
    int64_t *out_pos) {
    int fd;
    int w;
    int64_t pos;

    fd = host_handle_fd(h);
    if (fd < 0) {
        return DRACONIC_HOST_E_BADF;
    }
    if (whence == 0) {
        w = SEEK_SET;
    } else if (whence == 1) {
        w = SEEK_CUR;
    } else if (whence == 2) {
        w = SEEK_END;
    } else {
        return DRACONIC_HOST_E_INVAL;
    }
#if defined(_WIN32)
    pos = _lseeki64(fd, (long long)offset, w);
#else
    pos = (int64_t)lseek(fd, (off_t)offset, w);
#endif
    if (pos < 0) {
        return host_fs_errno_map();
    }
    if (out_pos) {
        *out_pos = pos;
    }
    return DRACONIC_HOST_OK;
}
