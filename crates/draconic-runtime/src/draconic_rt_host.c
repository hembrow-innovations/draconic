    /* Host I/O Runtime substrate (H00.02–H00.03, H01 process, H02.01 stdout,
    H04 fs, H06 TCP, H07 async, H08.01 UDP bind/sendto/recvfrom, H09 DNS,
    H10.01 HTTP/1.1 request parse, H10.02 response write, H11.01/H11.02 TLS).
    Error codes, opaque handles, UTF-8 path encoding, I/O bytes boundary,
    process, stdio, path, fs, TCP, UDP, DNS, HTTP, TLS, async readiness + Promise ops. */

#include "draconic_rt_host.h"

#if defined(__APPLE__)
#include <CoreFoundation/CoreFoundation.h>
#include <Security/SecureTransport.h>
#include <Security/Security.h>
#endif

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
#include <arpa/inet.h>
#include <dirent.h>
#include <netdb.h>
#include <netinet/in.h>
#include <poll.h>
#include <sys/socket.h>
#include <sys/time.h>
#include <signal.h>
#include <sys/wait.h>
#include <unistd.h>
/* setenv / unsetenv; getpid / getppid; fork/execvp/waitpid; mkdir / rmdir / unlink;
   open/read/write/lseek/close; socket/bind/listen/getsockname; getaddrinfo; poll */
#endif

/* Core job queue + Promise (draconic_rt.c) — H07.01/H07.02. */
typedef void (*DraconicJobFnHost)(void *data);
void draconic_rt_job_enqueue(DraconicJobFnHost fn, void *data);
DraconicValue *draconic_rt_promise_new(void);
void draconic_rt_promise_resolve(DraconicValue *p, void *value);
void draconic_rt_promise_reject(DraconicValue *p, void *reason);

/* --- Handle table (slots filled by open/listen/etc.) --- */

#define DRACONIC_HOST_HANDLE_SLOTS 256
#define DRACONIC_HOST_HANDLE_KIND_NONE 0
#define DRACONIC_HOST_HANDLE_KIND_FILE 1
#define DRACONIC_HOST_HANDLE_KIND_TCP_LISTEN 2
#define DRACONIC_HOST_HANDLE_KIND_TCP_CONN 3
#define DRACONIC_HOST_HANDLE_KIND_UDP 4
#define DRACONIC_HOST_HANDLE_KIND_TLS_CONN 5

/* Live flags + kind + OS fd for 1-based handle ids. */
static uint8_t g_host_handle_live[DRACONIC_HOST_HANDLE_SLOTS];
static uint8_t g_host_handle_kind[DRACONIC_HOST_HANDLE_SLOTS];
static int g_host_handle_fd[DRACONIC_HOST_HANDLE_SLOTS];
/* TLS: SSLContextRef (macOS) or NULL; parallel to handle slots. */
static void *g_host_handle_tls_ctx[DRACONIC_HOST_HANDLE_SLOTS];

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
            g_host_handle_tls_ctx[i] = NULL;
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

/* H07.01: cancel readiness waits for a handle (defined below). */
static void host_io_cancel_handle(DraconicHostHandle h);
/* H07.02: reject pending Promise async ops for a handle (defined below). */
static void host_tcp_async_cancel_handle(DraconicHostHandle h);

/* H11.01: tear down TLS context before closing fd (defined below). */
static void host_tls_ctx_dispose(void *ctx);

DraconicHostError draconic_rt_host_handle_close(DraconicHostHandle h) {
    size_t i;
    if (!draconic_rt_host_handle_is_valid(h)) {
        return DRACONIC_HOST_E_BADF;
    }
    i = (size_t)h - 1;
    host_tcp_async_cancel_handle(h);
    host_io_cancel_handle(h);
    if (g_host_handle_kind[i] == DRACONIC_HOST_HANDLE_KIND_TLS_CONN) {
        host_tls_ctx_dispose(g_host_handle_tls_ctx[i]);
        g_host_handle_tls_ctx[i] = NULL;
    }
    if (g_host_handle_kind[i] == DRACONIC_HOST_HANDLE_KIND_FILE
        || g_host_handle_kind[i] == DRACONIC_HOST_HANDLE_KIND_TCP_LISTEN
        || g_host_handle_kind[i] == DRACONIC_HOST_HANDLE_KIND_TCP_CONN
        || g_host_handle_kind[i] == DRACONIC_HOST_HANDLE_KIND_UDP
        || g_host_handle_kind[i] == DRACONIC_HOST_HANDLE_KIND_TLS_CONN) {
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
    g_host_handle_tls_ctx[i] = NULL;
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

/* --- Process run / spawn+wait (H15.01) --- */

int32_t draconic_rt_host_process_run(
    int32_t argc,
    const char **argv,
    const char *cwd,
    int32_t env_n,
    const char **env_keys,
    const char **env_vals) {
    int32_t i;

    if (argc < 1 || !argv || !argv[0]) {
        return -1;
    }
    for (i = 0; i < argc; i++) {
        if (!argv[i]) {
            return -1;
        }
    }
    if (env_n > 0) {
        if (!env_keys || !env_vals) {
            return -1;
        }
        for (i = 0; i < env_n; i++) {
            if (!env_keys[i] || !env_vals[i]) {
                return -1;
            }
        }
    }

#if defined(_WIN32)
    /* v1: posix-first; Windows CreateProcess path deferred. */
    (void)cwd;
    (void)env_n;
    (void)env_keys;
    (void)env_vals;
    return -1;
#else
    {
        char **av;
        pid_t child;
        int status;

        av = (char **)malloc((size_t)(argc + 1) * sizeof(char *));
        if (!av) {
            return -1;
        }
        for (i = 0; i < argc; i++) {
            av[i] = (char *)argv[i];
        }
        av[argc] = NULL;

        child = fork();
        if (child < 0) {
            free(av);
            return -1;
        }
        if (child == 0) {
            if (cwd && cwd[0] != '\0') {
                if (chdir(cwd) != 0) {
                    _exit(127);
                }
            }
            if (env_n > 0) {
                for (i = 0; i < env_n; i++) {
                    if (setenv(env_keys[i], env_vals[i], 1) != 0) {
                        _exit(127);
                    }
                }
            }
            execvp(av[0], av);
            _exit(127);
        }

        free(av);
        if (waitpid(child, &status, 0) < 0) {
            return -1;
        }
        if (WIFEXITED(status)) {
            return (int32_t)WEXITSTATUS(status);
        }
        if (WIFSIGNALED(status)) {
            return (int32_t)(128 + WTERMSIG(status));
        }
        return -1;
    }
#endif
}

/* --- Process spawn + pipes (H15.02) --- */

#define DRACONIC_PROC_SLOTS 32

typedef struct {
    uint8_t live;
    uint8_t waited;
    int stdin_fd;
    int stdout_fd;
    int stderr_fd;
#if !defined(_WIN32)
    pid_t pid;
#endif
    char *stdout_buf;
    char *stderr_buf;
    int32_t exit_code;
} DraconicProcSlot;

static DraconicProcSlot g_proc_slots[DRACONIC_PROC_SLOTS];

static void proc_slot_clear(DraconicProcSlot *s) {
    if (!s) {
        return;
    }
#if !defined(_WIN32)
    if (s->stdin_fd >= 0) {
        (void)close(s->stdin_fd);
    }
    if (s->stdout_fd >= 0) {
        (void)close(s->stdout_fd);
    }
    if (s->stderr_fd >= 0) {
        (void)close(s->stderr_fd);
    }
#endif
    free(s->stdout_buf);
    free(s->stderr_buf);
    s->live = 0;
    s->waited = 0;
    s->stdin_fd = -1;
    s->stdout_fd = -1;
    s->stderr_fd = -1;
#if !defined(_WIN32)
    s->pid = (pid_t)-1;
#endif
    s->stdout_buf = NULL;
    s->stderr_buf = NULL;
    s->exit_code = -1;
}

static DraconicProcSlot *proc_slot_get(int32_t h) {
    if (h < 1 || h > DRACONIC_PROC_SLOTS) {
        return NULL;
    }
    if (!g_proc_slots[(size_t)h - 1].live) {
        return NULL;
    }
    return &g_proc_slots[(size_t)h - 1];
}

#if !defined(_WIN32)
static int proc_buf_append(char **buf, size_t *len, size_t *cap, const char *tmp, size_t n) {
    if (*len + n + 1 > *cap) {
        size_t ncap = *cap ? *cap * 2 : 4096;
        char *nb;
        while (ncap < *len + n + 1) {
            ncap *= 2;
        }
        nb = (char *)realloc(*buf, ncap);
        if (!nb) {
            return -1;
        }
        *buf = nb;
        *cap = ncap;
    }
    memcpy(*buf + *len, tmp, n);
    *len += n;
    (*buf)[*len] = '\0';
    return 0;
}

/* Drain both pipes with poll to avoid stdout/stderr full-buffer deadlock. */
static int proc_drain_pipes(int out_fd, int err_fd, char **out_buf, char **err_buf) {
    size_t out_len = 0, out_cap = 0, err_len = 0, err_cap = 0;
    char *ob = NULL;
    char *eb = NULL;
    int out_open = out_fd >= 0;
    int err_open = err_fd >= 0;
    char tmp[4096];

    *out_buf = NULL;
    *err_buf = NULL;

    while (out_open || err_open) {
        struct pollfd pf[2];
        int nf = 0;
        int out_i = -1;
        int err_i = -1;
        int pr;

        if (out_open) {
            pf[nf].fd = out_fd;
            pf[nf].events = POLLIN;
            pf[nf].revents = 0;
            out_i = nf;
            nf++;
        }
        if (err_open) {
            pf[nf].fd = err_fd;
            pf[nf].events = POLLIN;
            pf[nf].revents = 0;
            err_i = nf;
            nf++;
        }
        pr = poll(pf, (nfds_t)nf, -1);
        if (pr < 0) {
            if (errno == EINTR) {
                continue;
            }
            free(ob);
            free(eb);
            return -1;
        }
        if (out_i >= 0 && (pf[out_i].revents & (POLLIN | POLLHUP | POLLERR))) {
            ssize_t n = read(out_fd, tmp, sizeof(tmp));
            if (n < 0) {
                if (errno == EINTR) {
                    continue;
                }
                free(ob);
                free(eb);
                return -1;
            }
            if (n == 0) {
                out_open = 0;
            } else if (proc_buf_append(&ob, &out_len, &out_cap, tmp, (size_t)n) != 0) {
                free(ob);
                free(eb);
                return -1;
            }
        }
        if (err_i >= 0 && (pf[err_i].revents & (POLLIN | POLLHUP | POLLERR))) {
            ssize_t n = read(err_fd, tmp, sizeof(tmp));
            if (n < 0) {
                if (errno == EINTR) {
                    continue;
                }
                free(ob);
                free(eb);
                return -1;
            }
            if (n == 0) {
                err_open = 0;
            } else if (proc_buf_append(&eb, &err_len, &err_cap, tmp, (size_t)n) != 0) {
                free(ob);
                free(eb);
                return -1;
            }
        }
    }

    if (!ob) {
        ob = (char *)malloc(1);
        if (!ob) {
            free(eb);
            return -1;
        }
        ob[0] = '\0';
    }
    if (!eb) {
        eb = (char *)malloc(1);
        if (!eb) {
            free(ob);
            return -1;
        }
        eb[0] = '\0';
    }
    *out_buf = ob;
    *err_buf = eb;
    return 0;
}
#endif

int32_t draconic_rt_host_process_spawn(
    int32_t argc,
    const char **argv,
    const char *cwd,
    int32_t env_n,
    const char **env_keys,
    const char **env_vals) {
    int32_t i;
    size_t slot;

    if (argc < 1 || !argv || !argv[0]) {
        return -1;
    }
    for (i = 0; i < argc; i++) {
        if (!argv[i]) {
            return -1;
        }
    }
    if (env_n > 0) {
        if (!env_keys || !env_vals) {
            return -1;
        }
        for (i = 0; i < env_n; i++) {
            if (!env_keys[i] || !env_vals[i]) {
                return -1;
            }
        }
    }

#if defined(_WIN32)
    (void)cwd;
    (void)env_n;
    (void)env_keys;
    (void)env_vals;
    return -1;
#else
    {
        int in_pipe[2] = { -1, -1 };
        int out_pipe[2] = { -1, -1 };
        int err_pipe[2] = { -1, -1 };
        char **av;
        pid_t child;
        DraconicProcSlot *s;

        for (slot = 0; slot < DRACONIC_PROC_SLOTS; slot++) {
            if (!g_proc_slots[slot].live) {
                break;
            }
        }
        if (slot >= DRACONIC_PROC_SLOTS) {
            return -1;
        }

        if (pipe(in_pipe) != 0 || pipe(out_pipe) != 0 || pipe(err_pipe) != 0) {
            if (in_pipe[0] >= 0) {
                close(in_pipe[0]);
                close(in_pipe[1]);
            }
            if (out_pipe[0] >= 0) {
                close(out_pipe[0]);
                close(out_pipe[1]);
            }
            if (err_pipe[0] >= 0) {
                close(err_pipe[0]);
                close(err_pipe[1]);
            }
            return -1;
        }

        av = (char **)malloc((size_t)(argc + 1) * sizeof(char *));
        if (!av) {
            close(in_pipe[0]);
            close(in_pipe[1]);
            close(out_pipe[0]);
            close(out_pipe[1]);
            close(err_pipe[0]);
            close(err_pipe[1]);
            return -1;
        }
        for (i = 0; i < argc; i++) {
            av[i] = (char *)argv[i];
        }
        av[argc] = NULL;

        child = fork();
        if (child < 0) {
            free(av);
            close(in_pipe[0]);
            close(in_pipe[1]);
            close(out_pipe[0]);
            close(out_pipe[1]);
            close(err_pipe[0]);
            close(err_pipe[1]);
            return -1;
        }
        if (child == 0) {
            close(in_pipe[1]);
            close(out_pipe[0]);
            close(err_pipe[0]);
            if (dup2(in_pipe[0], STDIN_FILENO) < 0) {
                _exit(127);
            }
            if (dup2(out_pipe[1], STDOUT_FILENO) < 0) {
                _exit(127);
            }
            if (dup2(err_pipe[1], STDERR_FILENO) < 0) {
                _exit(127);
            }
            close(in_pipe[0]);
            close(out_pipe[1]);
            close(err_pipe[1]);
            if (cwd && cwd[0] != '\0') {
                if (chdir(cwd) != 0) {
                    _exit(127);
                }
            }
            if (env_n > 0) {
                for (i = 0; i < env_n; i++) {
                    if (setenv(env_keys[i], env_vals[i], 1) != 0) {
                        _exit(127);
                    }
                }
            }
            execvp(av[0], av);
            _exit(127);
        }

        free(av);
        close(in_pipe[0]);
        close(out_pipe[1]);
        close(err_pipe[1]);

        s = &g_proc_slots[slot];
        proc_slot_clear(s);
        s->live = 1;
        s->pid = child;
        s->stdin_fd = in_pipe[1];
        s->stdout_fd = out_pipe[0];
        s->stderr_fd = err_pipe[0];
        s->waited = 0;
        s->exit_code = -1;
        return (int32_t)(slot + 1);
    }
#endif
}

int32_t draconic_rt_host_process_stdin_write(
    int32_t h,
    const char *data,
    int64_t len) {
    DraconicProcSlot *s = proc_slot_get(h);
    size_t n;
    size_t off = 0;

    if (!s || s->waited) {
        return -1;
    }
#if defined(_WIN32)
    (void)data;
    (void)len;
    return -1;
#else
    if (s->stdin_fd < 0) {
        return -1;
    }
    if (!data) {
        data = "";
        len = 0;
    }
    if (len < 0) {
        n = strlen(data);
    } else {
        n = (size_t)len;
    }
    while (off < n) {
        ssize_t w = write(s->stdin_fd, data + off, n - off);
        if (w < 0) {
            if (errno == EINTR) {
                continue;
            }
            close(s->stdin_fd);
            s->stdin_fd = -1;
            return -1;
        }
        off += (size_t)w;
    }
    close(s->stdin_fd);
    s->stdin_fd = -1;
    return 0;
#endif
}

/* Finish wait after waitpid collected status: drain pipes, set exit_code. */
#if !defined(_WIN32)
static void proc_finish_wait(DraconicProcSlot *s, int status) {
    if (s->stdin_fd >= 0) {
        close(s->stdin_fd);
        s->stdin_fd = -1;
    }
    (void)proc_drain_pipes(s->stdout_fd, s->stderr_fd, &s->stdout_buf, &s->stderr_buf);
    if (s->stdout_fd >= 0) {
        close(s->stdout_fd);
        s->stdout_fd = -1;
    }
    if (s->stderr_fd >= 0) {
        close(s->stderr_fd);
        s->stderr_fd = -1;
    }
    if (WIFEXITED(status)) {
        s->exit_code = (int32_t)WEXITSTATUS(status);
    } else if (WIFSIGNALED(status)) {
        s->exit_code = (int32_t)(128 + WTERMSIG(status));
    } else {
        s->exit_code = -1;
    }
    s->waited = 1;
}

/* 1 = exited (exit_code set), 0 = still running, -1 = error. */
static int proc_try_wait(DraconicProcSlot *s, int blocking) {
    int status;
    pid_t r;
    if (!s) {
        return -1;
    }
    if (s->waited) {
        return 1;
    }
    r = waitpid(s->pid, &status, blocking ? 0 : WNOHANG);
    if (r < 0) {
        return -1;
    }
    if (r == 0) {
        return 0;
    }
    proc_finish_wait(s, status);
    return 1;
}
#endif

typedef struct HostProcWaitOp {
    int32_t h;
    DraconicValue *promise;
    struct HostProcWaitOp *next;
} HostProcWaitOp;

static HostProcWaitOp *g_proc_waits;

static void *host_proc_async_num(int64_t n) {
    return (void *)(uintptr_t)(uint64_t)n;
}

static void host_proc_wait_settle(HostProcWaitOp *op, int32_t code, int ok) {
    if (!op) {
        return;
    }
    if (op->promise) {
        if (ok) {
            draconic_rt_promise_resolve(op->promise, host_proc_async_num((int64_t)code));
        } else {
            draconic_rt_promise_reject(op->promise, host_proc_async_num((int64_t)code));
        }
    }
    free(op);
}

static void host_proc_wait_cancel_handle(int32_t h) {
    HostProcWaitOp **pp = &g_proc_waits;
    while (*pp) {
        HostProcWaitOp *op = *pp;
        if (op->h == h) {
            *pp = op->next;
            host_proc_wait_settle(op, -1, 0);
        } else {
            pp = &op->next;
        }
    }
}

int32_t draconic_rt_host_process_wait(int32_t h) {
    DraconicProcSlot *s = proc_slot_get(h);

    if (!s) {
        return -1;
    }
    if (s->waited) {
        return s->exit_code;
    }
#if defined(_WIN32)
    return -1;
#else
    if (proc_try_wait(s, 1) != 1) {
        return -1;
    }
    return s->exit_code;
#endif
}

DraconicValue *draconic_rt_host_process_wait_async(int32_t h) {
    DraconicValue *p = draconic_rt_promise_new();
    DraconicProcSlot *s;
    HostProcWaitOp *op;
    int st;

    if (!p) {
        return NULL;
    }
#if defined(_WIN32)
    draconic_rt_promise_reject(p, host_proc_async_num((int64_t)-1));
    return p;
#else
    s = proc_slot_get(h);
    if (!s) {
        draconic_rt_promise_reject(p, host_proc_async_num((int64_t)-1));
        return p;
    }
    if (s->waited) {
        draconic_rt_promise_resolve(p, host_proc_async_num((int64_t)s->exit_code));
        return p;
    }
    st = proc_try_wait(s, 0);
    if (st == 1) {
        draconic_rt_promise_resolve(p, host_proc_async_num((int64_t)s->exit_code));
        return p;
    }
    if (st < 0) {
        draconic_rt_promise_reject(p, host_proc_async_num((int64_t)-1));
        return p;
    }
    op = (HostProcWaitOp *)malloc(sizeof(HostProcWaitOp));
    if (!op) {
        draconic_rt_promise_reject(p, host_proc_async_num((int64_t)-1));
        return p;
    }
    op->h = h;
    op->promise = p;
    op->next = g_proc_waits;
    g_proc_waits = op;
    return p;
#endif
}

int draconic_rt_host_process_pending(void) {
    return g_proc_waits != NULL ? 1 : 0;
}

int draconic_rt_host_process_poll(void) {
    int settled = 0;
#if !defined(_WIN32)
    HostProcWaitOp **pp = &g_proc_waits;
    while (*pp) {
        HostProcWaitOp *op = *pp;
        DraconicProcSlot *s = proc_slot_get(op->h);
        int st;
        if (!s) {
            *pp = op->next;
            host_proc_wait_settle(op, -1, 0);
            settled++;
            continue;
        }
        if (s->waited) {
            *pp = op->next;
            host_proc_wait_settle(op, s->exit_code, 1);
            settled++;
            continue;
        }
        st = proc_try_wait(s, 0);
        if (st == 1) {
            *pp = op->next;
            host_proc_wait_settle(op, s->exit_code, 1);
            settled++;
            continue;
        }
        if (st < 0) {
            *pp = op->next;
            host_proc_wait_settle(op, -1, 0);
            settled++;
            continue;
        }
        pp = &op->next;
    }
#endif
    return settled;
}

static int32_t proc_copy_buf(const char *src, char **out_text) {
    char *copy;
    size_t n;
    if (!out_text) {
        return (int32_t)DRACONIC_HOST_E_INVAL;
    }
    *out_text = NULL;
    if (!src) {
        src = "";
    }
    n = strlen(src);
    copy = (char *)malloc(n + 1);
    if (!copy) {
        return (int32_t)DRACONIC_HOST_E_NOMEM;
    }
    memcpy(copy, src, n + 1);
    *out_text = copy;
    return (int32_t)DRACONIC_HOST_OK;
}

int32_t draconic_rt_host_process_stdout(int32_t h, char **out_text) {
    DraconicProcSlot *s = proc_slot_get(h);
    if (!s || !s->waited) {
        if (out_text) {
            *out_text = NULL;
        }
        return (int32_t)DRACONIC_HOST_E_INVAL;
    }
    return proc_copy_buf(s->stdout_buf, out_text);
}

int32_t draconic_rt_host_process_stderr(int32_t h, char **out_text) {
    DraconicProcSlot *s = proc_slot_get(h);
    if (!s || !s->waited) {
        if (out_text) {
            *out_text = NULL;
        }
        return (int32_t)DRACONIC_HOST_E_INVAL;
    }
    return proc_copy_buf(s->stderr_buf, out_text);
}

int32_t draconic_rt_host_process_kill(int32_t h) {
    DraconicProcSlot *s = proc_slot_get(h);
    if (!s || s->waited) {
        return -1;
    }
#if defined(_WIN32)
    return -1;
#else
    if (kill(s->pid, SIGTERM) != 0) {
        return -1;
    }
    return 0;
#endif
}

int32_t draconic_rt_host_process_close(int32_t h) {
    DraconicProcSlot *s = proc_slot_get(h);
    if (!s) {
        return -1;
    }
    host_proc_wait_cancel_handle(h);
#if !defined(_WIN32)
    if (!s->waited && s->pid > 0) {
        (void)kill(s->pid, SIGKILL);
        (void)waitpid(s->pid, NULL, 0);
    }
#endif
    proc_slot_clear(s);
    return 0;
}

/* --- Process signals (H14.01 / H14.02) ---
   Default terminate: when no watch/ignore is installed, OS SIG_DFL remains
   (process terminates on SIGINT/SIGTERM). Watch installs a SA_RESTART handler
   that only sets a flag; draconic_rt_host_signal_poll enqueues the user job.
   Ignore installs SIG_IGN; restore installs SIG_DFL and clears watch state. */

#if !defined(_WIN32)
#include <signal.h>
#endif

#if defined(_WIN32)
static volatile int g_sigint_pending;
static volatile int g_sigterm_pending;
#else
static volatile sig_atomic_t g_sigint_pending;
static volatile sig_atomic_t g_sigterm_pending;
#endif
static DraconicHostSignalFn g_sigint_fn;
static void *g_sigint_data;
static DraconicHostSignalFn g_sigterm_fn;
static void *g_sigterm_data;
static int g_sigint_watched;
static int g_sigterm_watched;

#if !defined(_WIN32)
static void host_signal_handler(int signo) {
    if (signo == SIGINT) {
        g_sigint_pending = 1;
    } else if (signo == SIGTERM) {
        g_sigterm_pending = 1;
    }
}

static int host_sig_os(int32_t sig) {
    if (sig == DRACONIC_HOST_SIG_INT) {
        return SIGINT;
    }
    if (sig == DRACONIC_HOST_SIG_TERM) {
        return SIGTERM;
    }
    return -1;
}

static void host_signal_clear_watch(int32_t sig) {
    if (sig == DRACONIC_HOST_SIG_INT) {
        g_sigint_fn = NULL;
        g_sigint_data = NULL;
        g_sigint_watched = 0;
        g_sigint_pending = 0;
    } else {
        g_sigterm_fn = NULL;
        g_sigterm_data = NULL;
        g_sigterm_watched = 0;
        g_sigterm_pending = 0;
    }
}

static DraconicHostError host_signal_set_action(int32_t sig, void (*handler)(int)) {
    int os = host_sig_os(sig);
    struct sigaction sa;
    if (os < 0) {
        return DRACONIC_HOST_E_INVAL;
    }
    memset(&sa, 0, sizeof(sa));
    sa.sa_handler = handler;
    sigemptyset(&sa.sa_mask);
    sa.sa_flags = SA_RESTART;
    if (sigaction(os, &sa, NULL) != 0) {
        return DRACONIC_HOST_E_IO;
    }
    return DRACONIC_HOST_OK;
}
#endif

DraconicHostError draconic_rt_host_signal_watch(
    int32_t sig,
    DraconicHostSignalFn fn,
    void *data
) {
    if (!fn) {
        return DRACONIC_HOST_E_INVAL;
    }
#if defined(_WIN32)
    (void)sig;
    (void)data;
    return DRACONIC_HOST_E_NOSYS;
#else
    {
        DraconicHostError err;
        if (host_sig_os(sig) < 0) {
            return DRACONIC_HOST_E_INVAL;
        }
        if (sig == DRACONIC_HOST_SIG_INT) {
            g_sigint_fn = fn;
            g_sigint_data = data;
            g_sigint_watched = 1;
        } else {
            g_sigterm_fn = fn;
            g_sigterm_data = data;
            g_sigterm_watched = 1;
        }
        err = host_signal_set_action(sig, host_signal_handler);
        if (err != DRACONIC_HOST_OK) {
            host_signal_clear_watch(sig);
            return err;
        }
        return DRACONIC_HOST_OK;
    }
#endif
}

DraconicHostError draconic_rt_host_signal_ignore(int32_t sig) {
#if defined(_WIN32)
    (void)sig;
    return DRACONIC_HOST_E_NOSYS;
#else
    {
        if (host_sig_os(sig) < 0) {
            return DRACONIC_HOST_E_INVAL;
        }
        host_signal_clear_watch(sig);
        return host_signal_set_action(sig, SIG_IGN);
    }
#endif
}

DraconicHostError draconic_rt_host_signal_restore(int32_t sig) {
#if defined(_WIN32)
    (void)sig;
    return DRACONIC_HOST_E_NOSYS;
#else
    {
        if (host_sig_os(sig) < 0) {
            return DRACONIC_HOST_E_INVAL;
        }
        host_signal_clear_watch(sig);
        return host_signal_set_action(sig, SIG_DFL);
    }
#endif
}

DraconicHostError draconic_rt_host_signal_raise(int32_t sig) {
#if defined(_WIN32)
    (void)sig;
    return DRACONIC_HOST_E_NOSYS;
#else
    {
        int os = host_sig_os(sig);
        if (os < 0) {
            return DRACONIC_HOST_E_INVAL;
        }
        if (raise(os) != 0) {
            return DRACONIC_HOST_E_IO;
        }
        return DRACONIC_HOST_OK;
    }
#endif
}

int draconic_rt_host_signal_poll(void) {
    int n = 0;
    if (g_sigint_pending) {
        g_sigint_pending = 0;
        if (g_sigint_watched && g_sigint_fn) {
            draconic_rt_job_enqueue(g_sigint_fn, g_sigint_data);
            n++;
        }
    }
    if (g_sigterm_pending) {
        g_sigterm_pending = 0;
        if (g_sigterm_watched && g_sigterm_fn) {
            draconic_rt_job_enqueue(g_sigterm_fn, g_sigterm_data);
            n++;
        }
    }
    return n;
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

/* --- TCP listen/accept/connect/peer/io (H06.01–H06.04) ------------------- */

#if !defined(_WIN32)
static DraconicHostError host_tcp_errno_map(void) {
    if (errno == EADDRINUSE || errno == EADDRNOTAVAIL) {
        return DRACONIC_HOST_E_ADDR;
    }
    /* H06.03: refused / reset / unreachable / timed-out connect → E_CONN. */
    if (errno == ECONNREFUSED || errno == ECONNRESET || errno == ENETUNREACH
        || errno == EHOSTUNREACH || errno == ETIMEDOUT) {
        return DRACONIC_HOST_E_CONN;
    }
    if (errno == EACCES
#if defined(EPERM)
        || errno == EPERM
#endif
    ) {
        return DRACONIC_HOST_E_PERM;
    }
    if (errno == ENOMEM
#if defined(ENOBUFS)
        || errno == ENOBUFS
#endif
    ) {
        return DRACONIC_HOST_E_NOMEM;
    }
    if (errno == EAGAIN || errno == EWOULDBLOCK) {
        return DRACONIC_HOST_E_AGAIN;
    }
    if (errno == EINVAL) {
        return DRACONIC_HOST_E_INVAL;
    }
    return DRACONIC_HOST_E_IO;
}

static int host_handle_tcp_listen_fd(DraconicHostHandle h) {
    if (!draconic_rt_host_handle_is_valid(h)) {
        return -1;
    }
    if (g_host_handle_kind[(size_t)h - 1] != DRACONIC_HOST_HANDLE_KIND_TCP_LISTEN) {
        return -1;
    }
    return g_host_handle_fd[(size_t)h - 1];
}

static int host_handle_tcp_conn_fd(DraconicHostHandle h) {
    if (!draconic_rt_host_handle_is_valid(h)) {
        return -1;
    }
    if (g_host_handle_kind[(size_t)h - 1] != DRACONIC_HOST_HANDLE_KIND_TCP_CONN) {
        return -1;
    }
    return g_host_handle_fd[(size_t)h - 1];
}
#endif

DraconicHostError draconic_rt_host_tcp_listen(
    int32_t port,
    int32_t backlog,
    DraconicHostHandle *out_h) {
#if defined(_WIN32)
    (void)port;
    (void)backlog;
    (void)out_h;
    return DRACONIC_HOST_E_NOSYS;
#else
    int fd = -1;
    int yes = 1;
    struct sockaddr_in addr;
    DraconicHostError err;

    if (!out_h) {
        return DRACONIC_HOST_E_INVAL;
    }
    *out_h = DRACONIC_HOST_HANDLE_INVALID;
    if (port < 0 || port > 65535) {
        return DRACONIC_HOST_E_INVAL;
    }
    if (backlog <= 0) {
        backlog = 128;
    }

    fd = (int)socket(AF_INET, SOCK_STREAM, 0);
    if (fd < 0) {
        return host_tcp_errno_map();
    }

    if (setsockopt(fd, SOL_SOCKET, SO_REUSEADDR, &yes, sizeof(yes)) < 0) {
        err = host_tcp_errno_map();
        (void)close(fd);
        return err;
    }

    memset(&addr, 0, sizeof(addr));
    addr.sin_family = AF_INET;
    addr.sin_addr.s_addr = htonl(INADDR_ANY);
    addr.sin_port = htons((uint16_t)port);

    if (bind(fd, (struct sockaddr *)&addr, sizeof(addr)) < 0) {
        err = host_tcp_errno_map();
        (void)close(fd);
        return err;
    }

    if (listen(fd, backlog) < 0) {
        err = host_tcp_errno_map();
        (void)close(fd);
        return err;
    }

    err = host_handle_alloc(DRACONIC_HOST_HANDLE_KIND_TCP_LISTEN, fd, out_h);
    if (err != DRACONIC_HOST_OK) {
        (void)close(fd);
        return err;
    }
    return DRACONIC_HOST_OK;
#endif
}

DraconicHostError draconic_rt_host_tcp_local_port(
    DraconicHostHandle h,
    int32_t *out_port) {
#if defined(_WIN32)
    (void)h;
    (void)out_port;
    return DRACONIC_HOST_E_NOSYS;
#else
    int fd;
    struct sockaddr_in addr;
    socklen_t len = (socklen_t)sizeof(addr);

    if (!out_port) {
        return DRACONIC_HOST_E_INVAL;
    }
    *out_port = 0;
    fd = host_handle_tcp_listen_fd(h);
    if (fd < 0) {
        return DRACONIC_HOST_E_BADF;
    }
    memset(&addr, 0, sizeof(addr));
    if (getsockname(fd, (struct sockaddr *)&addr, &len) < 0) {
        return host_tcp_errno_map();
    }
    *out_port = (int32_t)ntohs(addr.sin_port);
    return DRACONIC_HOST_OK;
#endif
}

DraconicHostError draconic_rt_host_tcp_accept(
    DraconicHostHandle listen_h,
    DraconicHostHandle *out_conn) {
#if defined(_WIN32)
    (void)listen_h;
    (void)out_conn;
    return DRACONIC_HOST_E_NOSYS;
#else
    int lfd;
    int cfd;
    struct sockaddr_in peer;
    socklen_t plen = (socklen_t)sizeof(peer);
    DraconicHostError err;

    if (!out_conn) {
        return DRACONIC_HOST_E_INVAL;
    }
    *out_conn = DRACONIC_HOST_HANDLE_INVALID;
    lfd = host_handle_tcp_listen_fd(listen_h);
    if (lfd < 0) {
        return DRACONIC_HOST_E_BADF;
    }
    memset(&peer, 0, sizeof(peer));
    cfd = (int)accept(lfd, (struct sockaddr *)&peer, &plen);
    if (cfd < 0) {
        return host_tcp_errno_map();
    }
    err = host_handle_alloc(DRACONIC_HOST_HANDLE_KIND_TCP_CONN, cfd, out_conn);
    if (err != DRACONIC_HOST_OK) {
        (void)close(cfd);
        return err;
    }
    return DRACONIC_HOST_OK;
#endif
}

#if !defined(_WIN32)
/* H09.02: resolve hostname or IPv4 dotted literal → first AF_INET addr.
   Empty/NULL → INVAL; getaddrinfo failure / no A → E_ADDR. */
static DraconicHostError host_resolve_ipv4(const char *host, struct in_addr *out) {
    struct addrinfo hints;
    struct addrinfo *res = NULL;
    struct addrinfo *rp;
    int gai;

    if (!host || host[0] == '\0' || !out) {
        return DRACONIC_HOST_E_INVAL;
    }
    if (inet_pton(AF_INET, host, out) == 1) {
        return DRACONIC_HOST_OK;
    }
    memset(&hints, 0, sizeof(hints));
    hints.ai_family = AF_INET;
    hints.ai_socktype = SOCK_STREAM;
    gai = getaddrinfo(host, NULL, &hints, &res);
    if (gai != 0) {
        return DRACONIC_HOST_E_ADDR;
    }
    for (rp = res; rp != NULL; rp = rp->ai_next) {
        struct sockaddr_in *sa;
        if (rp->ai_family != AF_INET || !rp->ai_addr) {
            continue;
        }
        sa = (struct sockaddr_in *)rp->ai_addr;
        *out = sa->sin_addr;
        freeaddrinfo(res);
        return DRACONIC_HOST_OK;
    }
    freeaddrinfo(res);
    return DRACONIC_HOST_E_ADDR;
}
#endif

DraconicHostError draconic_rt_host_tcp_connect(
    const char *host,
    int32_t port,
    DraconicHostHandle *out_conn) {
#if defined(_WIN32)
    (void)host;
    (void)port;
    (void)out_conn;
    return DRACONIC_HOST_E_NOSYS;
#else
    int fd = -1;
    struct sockaddr_in addr;
    DraconicHostError err;

    if (!out_conn) {
        return DRACONIC_HOST_E_INVAL;
    }
    *out_conn = DRACONIC_HOST_HANDLE_INVALID;
    if (!host || host[0] == '\0' || port < 1 || port > 65535) {
        return DRACONIC_HOST_E_INVAL;
    }

    memset(&addr, 0, sizeof(addr));
    addr.sin_family = AF_INET;
    addr.sin_port = htons((uint16_t)port);
    err = host_resolve_ipv4(host, &addr.sin_addr);
    if (err != DRACONIC_HOST_OK) {
        return err;
    }

    fd = (int)socket(AF_INET, SOCK_STREAM, 0);
    if (fd < 0) {
        return host_tcp_errno_map();
    }
    if (connect(fd, (struct sockaddr *)&addr, sizeof(addr)) < 0) {
        err = host_tcp_errno_map();
        (void)close(fd);
        return err;
    }
    err = host_handle_alloc(DRACONIC_HOST_HANDLE_KIND_TCP_CONN, fd, out_conn);
    if (err != DRACONIC_HOST_OK) {
        (void)close(fd);
        return err;
    }
    return DRACONIC_HOST_OK;
#endif
}

DraconicHostError draconic_rt_host_tcp_peer_port(
    DraconicHostHandle conn_h,
    int32_t *out_port) {
#if defined(_WIN32)
    (void)conn_h;
    (void)out_port;
    return DRACONIC_HOST_E_NOSYS;
#else
    int fd;
    struct sockaddr_in addr;
    socklen_t len = (socklen_t)sizeof(addr);

    if (!out_port) {
        return DRACONIC_HOST_E_INVAL;
    }
    *out_port = 0;
    fd = host_handle_tcp_conn_fd(conn_h);
    if (fd < 0) {
        return DRACONIC_HOST_E_BADF;
    }
    memset(&addr, 0, sizeof(addr));
    if (getpeername(fd, (struct sockaddr *)&addr, &len) < 0) {
        return host_tcp_errno_map();
    }
    *out_port = (int32_t)ntohs(addr.sin_port);
    return DRACONIC_HOST_OK;
#endif
}

DraconicHostError draconic_rt_host_tcp_peer_address(
    DraconicHostHandle conn_h,
    char **out_addr) {
#if defined(_WIN32)
    (void)conn_h;
    (void)out_addr;
    return DRACONIC_HOST_E_NOSYS;
#else
    int fd;
    struct sockaddr_in addr;
    socklen_t len = (socklen_t)sizeof(addr);
    char buf[INET_ADDRSTRLEN];
    char *dup;

    if (!out_addr) {
        return DRACONIC_HOST_E_INVAL;
    }
    *out_addr = NULL;
    fd = host_handle_tcp_conn_fd(conn_h);
    if (fd < 0) {
        return DRACONIC_HOST_E_BADF;
    }
    memset(&addr, 0, sizeof(addr));
    if (getpeername(fd, (struct sockaddr *)&addr, &len) < 0) {
        return host_tcp_errno_map();
    }
    if (!inet_ntop(AF_INET, &addr.sin_addr, buf, sizeof(buf))) {
        return host_tcp_errno_map();
    }
    dup = (char *)malloc(strlen(buf) + 1);
    if (!dup) {
        return DRACONIC_HOST_E_NOMEM;
    }
    memcpy(dup, buf, strlen(buf) + 1);
    *out_addr = dup;
    return DRACONIC_HOST_OK;
#endif
}

DraconicHostError draconic_rt_host_tcp_read(
    DraconicHostHandle conn_h,
    size_t max_len,
    uint8_t **out_data,
    size_t *out_len) {
#if defined(_WIN32)
    (void)conn_h;
    (void)max_len;
    (void)out_data;
    (void)out_len;
    return DRACONIC_HOST_E_NOSYS;
#else
    int fd;
    uint8_t *buf = NULL;
    size_t got = 0;
    ssize_t n;

    if (!out_data || !out_len) {
        return DRACONIC_HOST_E_INVAL;
    }
    *out_data = NULL;
    *out_len = 0;
    fd = host_handle_tcp_conn_fd(conn_h);
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
    n = read(fd, buf, max_len);
    if (n < 0) {
        free(buf);
        return host_tcp_errno_map();
    }
    got = (size_t)n;
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
#endif
}

DraconicHostError draconic_rt_host_tcp_write(
    DraconicHostHandle conn_h,
    const uint8_t *data,
    size_t len) {
#if defined(_WIN32)
    (void)conn_h;
    (void)data;
    (void)len;
    return DRACONIC_HOST_E_NOSYS;
#else
    int fd;
    size_t off = 0;

    fd = host_handle_tcp_conn_fd(conn_h);
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
        ssize_t n = write(fd, data + off, len - off);
        if (n < 0) {
            return host_tcp_errno_map();
        }
        if (n == 0) {
            return DRACONIC_HOST_E_IO;
        }
        off += (size_t)n;
    }
    return DRACONIC_HOST_OK;
#endif
}

DraconicHostError draconic_rt_host_tcp_shutdown(
    DraconicHostHandle conn_h,
    int32_t how) {
#if defined(_WIN32)
    (void)conn_h;
    (void)how;
    return DRACONIC_HOST_E_NOSYS;
#else
    int fd;
    int sh;

    fd = host_handle_tcp_conn_fd(conn_h);
    if (fd < 0) {
        return DRACONIC_HOST_E_BADF;
    }
    /* Map 0/1/2 → SHUT_RD/SHUT_WR/SHUT_RDWR. */
    if (how == 0) {
        sh = SHUT_RD;
    } else if (how == 1) {
        sh = SHUT_WR;
    } else if (how == 2) {
        sh = SHUT_RDWR;
    } else {
        return DRACONIC_HOST_E_INVAL;
    }
    if (shutdown(fd, sh) < 0) {
        return host_tcp_errno_map();
    }
    return DRACONIC_HOST_OK;
#endif
}

/* --- Async socket readiness (H07.01) ------------------------------------- */

typedef struct HostIoWait {
    int64_t id;
    DraconicHostHandle h;
    int32_t events;
    DraconicHostIoFn fn;
    void *data;
    int cancelled;
    int enqueued;
    struct HostIoWait *next;
} HostIoWait;

static HostIoWait *g_io_wait_head = NULL;
static int64_t g_io_wait_next_id = 1;

static void host_io_unlink_and_free(HostIoWait *target) {
    HostIoWait **link = &g_io_wait_head;
    while (*link) {
        if (*link == target) {
            *link = target->next;
            free(target);
            return;
        }
        link = &(*link)->next;
    }
    free(target);
}

static void host_io_wait_job(void *data) {
    HostIoWait *w = (HostIoWait *)data;
    if (w && !w->cancelled && w->fn) {
        w->fn(w->data);
    }
    if (w) {
        host_io_unlink_and_free(w);
    }
}

static void host_io_cancel_handle(DraconicHostHandle h) {
    HostIoWait *w = g_io_wait_head;
    while (w) {
        HostIoWait *next = w->next;
        if (w->h == h) {
            w->cancelled = 1;
            if (!w->enqueued) {
                host_io_unlink_and_free(w);
            }
        }
        w = next;
    }
}

#if !defined(_WIN32)
static int host_handle_tcp_any_fd(DraconicHostHandle h) {
    if (!draconic_rt_host_handle_is_valid(h)) {
        return -1;
    }
    {
        uint8_t kind = g_host_handle_kind[(size_t)h - 1];
        if (kind != DRACONIC_HOST_HANDLE_KIND_TCP_LISTEN
            && kind != DRACONIC_HOST_HANDLE_KIND_TCP_CONN) {
            return -1;
        }
    }
    return g_host_handle_fd[(size_t)h - 1];
}
#endif

DraconicHostError draconic_rt_host_tcp_set_nonblocking(
    DraconicHostHandle h,
    int32_t enable) {
#if defined(_WIN32)
    (void)h;
    (void)enable;
    return DRACONIC_HOST_E_NOSYS;
#else
    int fd;
    int flags;
    fd = host_handle_tcp_any_fd(h);
    if (fd < 0) {
        return DRACONIC_HOST_E_BADF;
    }
    flags = fcntl(fd, F_GETFL, 0);
    if (flags < 0) {
        return host_tcp_errno_map();
    }
    if (enable) {
        flags |= O_NONBLOCK;
    } else {
        flags &= ~O_NONBLOCK;
    }
    if (fcntl(fd, F_SETFL, flags) < 0) {
        return host_tcp_errno_map();
    }
    return DRACONIC_HOST_OK;
#endif
}

DraconicHostError draconic_rt_host_io_wait(
    DraconicHostHandle h,
    int32_t events,
    DraconicHostIoFn fn,
    void *data,
    int64_t *out_id) {
#if defined(_WIN32)
    (void)h;
    (void)events;
    (void)fn;
    (void)data;
    (void)out_id;
    return DRACONIC_HOST_E_NOSYS;
#else
    HostIoWait *w;
    int fd;

    if (!out_id) {
        return DRACONIC_HOST_E_INVAL;
    }
    *out_id = 0;
    if (!fn) {
        return DRACONIC_HOST_E_INVAL;
    }
    if ((events & (DRACONIC_HOST_IO_READ | DRACONIC_HOST_IO_WRITE)) == 0) {
        return DRACONIC_HOST_E_INVAL;
    }
    if ((events & ~(DRACONIC_HOST_IO_READ | DRACONIC_HOST_IO_WRITE)) != 0) {
        return DRACONIC_HOST_E_INVAL;
    }
    fd = host_handle_tcp_any_fd(h);
    if (fd < 0) {
        return DRACONIC_HOST_E_BADF;
    }
    (void)fd;
    w = (HostIoWait *)calloc(1, sizeof(HostIoWait));
    if (!w) {
        return DRACONIC_HOST_E_NOMEM;
    }
    w->id = g_io_wait_next_id++;
    if (g_io_wait_next_id <= 0) {
        g_io_wait_next_id = 1;
    }
    w->h = h;
    w->events = events;
    w->fn = fn;
    w->data = data;
    w->cancelled = 0;
    w->enqueued = 0;
    w->next = g_io_wait_head;
    g_io_wait_head = w;
    *out_id = w->id;
    return DRACONIC_HOST_OK;
#endif
}

void draconic_rt_host_io_cancel(int64_t id) {
    if (id <= 0) {
        return;
    }
    for (HostIoWait *w = g_io_wait_head; w; w = w->next) {
        if (w->id == id) {
            w->cancelled = 1;
            if (!w->enqueued) {
                host_io_unlink_and_free(w);
            }
            return;
        }
    }
}

int draconic_rt_host_io_pending(void) {
    for (HostIoWait *w = g_io_wait_head; w; w = w->next) {
        if (!w->cancelled && !w->enqueued) {
            return 1;
        }
    }
    return 0;
}

int draconic_rt_host_io_poll(double timeout_ms) {
#if defined(_WIN32)
    (void)timeout_ms;
    return 0;
#else
    HostIoWait *list[DRACONIC_HOST_HANDLE_SLOTS];
    struct pollfd pfds[DRACONIC_HOST_HANDLE_SLOTS];
    int nwait = 0;
    int timeout_i;
    int pr;
    int completed = 0;
    HostIoWait *w;

    for (w = g_io_wait_head; w; w = w->next) {
        int fd;
        short ev = 0;
        if (w->cancelled || w->enqueued) {
            continue;
        }
        fd = host_handle_tcp_any_fd(w->h);
        if (fd < 0) {
            w->cancelled = 1;
            continue;
        }
        if (nwait >= (int)DRACONIC_HOST_HANDLE_SLOTS) {
            break;
        }
        if (w->events & DRACONIC_HOST_IO_READ) {
            ev = (short)(ev | POLLIN);
        }
        if (w->events & DRACONIC_HOST_IO_WRITE) {
            ev = (short)(ev | POLLOUT);
        }
        list[nwait] = w;
        pfds[nwait].fd = fd;
        pfds[nwait].events = ev;
        pfds[nwait].revents = 0;
        nwait++;
    }

    if (nwait == 0) {
        /* Drop cancelled not-yet-enqueued waits. */
        w = g_io_wait_head;
        while (w) {
            HostIoWait *next = w->next;
            if (w->cancelled && !w->enqueued) {
                host_io_unlink_and_free(w);
            }
            w = next;
        }
        return 0;
    }

    if (timeout_ms < 0.0 || timeout_ms != timeout_ms) {
        timeout_i = -1;
    } else if (timeout_ms == 0.0) {
        timeout_i = 0;
    } else {
        if (timeout_ms > 60000.0) {
            timeout_ms = 60000.0;
        }
        timeout_i = (int)(timeout_ms + 0.5);
        if (timeout_i < 1) {
            timeout_i = 1;
        }
    }

    pr = poll(pfds, (nfds_t)nwait, timeout_i);
    if (pr < 0) {
        if (errno == EINTR) {
            return 0;
        }
        return 0;
    }
    if (pr == 0) {
        return 0;
    }

    for (int i = 0; i < nwait; i++) {
        short rev = pfds[i].revents;
        int ready = 0;
        w = list[i];
        if (!w || w->cancelled || w->enqueued) {
            continue;
        }
        if (rev & (POLLERR | POLLHUP | POLLNVAL)) {
            ready = 1;
        } else {
            if ((w->events & DRACONIC_HOST_IO_READ) && (rev & POLLIN)) {
                ready = 1;
            }
            if ((w->events & DRACONIC_HOST_IO_WRITE) && (rev & POLLOUT)) {
                ready = 1;
            }
        }
        if (!ready) {
            continue;
        }
        w->enqueued = 1;
        draconic_rt_job_enqueue(host_io_wait_job, w);
        completed++;
    }
    return completed;
#endif
}

/* --- Async TCP → Promises (H07.02) --------------------------------------- */

enum {
    HOST_TCP_ASYNC_ACCEPT = 1,
    HOST_TCP_ASYNC_CONNECT = 2,
    HOST_TCP_ASYNC_READ = 3,
    HOST_TCP_ASYNC_WRITE = 4
};

typedef struct HostTcpAsyncOp {
    int kind;
    DraconicValue *promise;
    DraconicHostHandle h;
    int64_t wait_id;
    int settled;
    /* CONNECT: socket already allocated as conn handle; host string retained. */
    char *connect_host;
    int32_t connect_port;
    /* READ */
    int64_t max_len;
    /* WRITE: owned copy of payload until settle. */
    uint8_t *write_data;
    size_t write_len;
    size_t write_off;
    struct HostTcpAsyncOp *next;
} HostTcpAsyncOp;

static HostTcpAsyncOp *g_tcp_async_ops = NULL;

static void *host_tcp_async_num(int64_t n) {
    return (void *)(intptr_t)n;
}

static void host_tcp_async_unlink(HostTcpAsyncOp *target) {
    HostTcpAsyncOp **link = &g_tcp_async_ops;
    while (*link) {
        if (*link == target) {
            *link = target->next;
            return;
        }
        link = &(*link)->next;
    }
}

static void host_tcp_async_free(HostTcpAsyncOp *op) {
    if (!op) {
        return;
    }
    free(op->connect_host);
    free(op->write_data);
    free(op);
}

static void host_tcp_async_settle_ok(HostTcpAsyncOp *op, void *value) {
    if (!op || op->settled) {
        return;
    }
    op->settled = 1;
    if (op->wait_id > 0) {
        draconic_rt_host_io_cancel(op->wait_id);
        op->wait_id = 0;
    }
    if (op->promise) {
        draconic_rt_promise_resolve(op->promise, value);
    }
    host_tcp_async_unlink(op);
    host_tcp_async_free(op);
}

static void host_tcp_async_settle_err(HostTcpAsyncOp *op, DraconicHostError err) {
    if (!op || op->settled) {
        return;
    }
    op->settled = 1;
    if (op->wait_id > 0) {
        draconic_rt_host_io_cancel(op->wait_id);
        op->wait_id = 0;
    }
    if (op->promise) {
        draconic_rt_promise_reject(op->promise, host_tcp_async_num((int64_t)err));
    }
    host_tcp_async_unlink(op);
    host_tcp_async_free(op);
}

static void host_tcp_async_cancel_handle(DraconicHostHandle h) {
    HostTcpAsyncOp *op = g_tcp_async_ops;
    while (op) {
        HostTcpAsyncOp *next = op->next;
        if (!op->settled && op->h == h) {
            host_tcp_async_settle_err(op, DRACONIC_HOST_E_BADF);
        }
        op = next;
    }
}

static HostTcpAsyncOp *host_tcp_async_alloc(
    int kind,
    DraconicHostHandle h,
    DraconicValue *promise) {
    HostTcpAsyncOp *op = (HostTcpAsyncOp *)calloc(1, sizeof(HostTcpAsyncOp));
    if (!op) {
        return NULL;
    }
    op->kind = kind;
    op->h = h;
    op->promise = promise;
    op->next = g_tcp_async_ops;
    g_tcp_async_ops = op;
    return op;
}

static void host_tcp_async_on_accept_ready(void *data) {
    HostTcpAsyncOp *op = (HostTcpAsyncOp *)data;
    DraconicHostHandle conn = DRACONIC_HOST_HANDLE_INVALID;
    DraconicHostError err;
    if (!op || op->settled) {
        return;
    }
    err = draconic_rt_host_tcp_accept(op->h, &conn);
    if (err == DRACONIC_HOST_E_AGAIN) {
        /* Spurious wake: re-arm. */
        int64_t id = 0;
        err = draconic_rt_host_io_wait(
            op->h, DRACONIC_HOST_IO_READ, host_tcp_async_on_accept_ready, op, &id);
        if (err != DRACONIC_HOST_OK) {
            host_tcp_async_settle_err(op, err);
            return;
        }
        op->wait_id = id;
        return;
    }
    if (err != DRACONIC_HOST_OK) {
        host_tcp_async_settle_err(op, err);
        return;
    }
    host_tcp_async_settle_ok(op, host_tcp_async_num((int64_t)conn));
}

static void host_tcp_async_on_connect_ready(void *data) {
#if defined(_WIN32)
    (void)data;
#else
    HostTcpAsyncOp *op = (HostTcpAsyncOp *)data;
    int fd;
    int soerr = 0;
    socklen_t slen = (socklen_t)sizeof(soerr);
    DraconicHostHandle h;
    if (!op || op->settled) {
        return;
    }
    h = op->h;
    fd = host_handle_tcp_conn_fd(h);
    if (fd < 0) {
        host_tcp_async_settle_err(op, DRACONIC_HOST_E_BADF);
        return;
    }
    if (getsockopt(fd, SOL_SOCKET, SO_ERROR, &soerr, &slen) < 0) {
        host_tcp_async_settle_err(op, host_tcp_errno_map());
        (void)draconic_rt_host_handle_close(h);
        return;
    }
    if (soerr != 0) {
        errno = soerr;
        host_tcp_async_settle_err(op, host_tcp_errno_map());
        (void)draconic_rt_host_handle_close(h);
        return;
    }
    host_tcp_async_settle_ok(op, host_tcp_async_num((int64_t)h));
#endif
}

static void host_tcp_async_on_read_ready(void *data) {
    HostTcpAsyncOp *op = (HostTcpAsyncOp *)data;
    uint8_t *buf = NULL;
    size_t len = 0;
    DraconicHostError err;
    if (!op || op->settled) {
        return;
    }
    err = draconic_rt_host_tcp_read(op->h, (size_t)op->max_len, &buf, &len);
    if (err == DRACONIC_HOST_E_AGAIN) {
        int64_t id = 0;
        err = draconic_rt_host_io_wait(
            op->h, DRACONIC_HOST_IO_READ, host_tcp_async_on_read_ready, op, &id);
        if (err != DRACONIC_HOST_OK) {
            host_tcp_async_settle_err(op, err);
            return;
        }
        op->wait_id = id;
        return;
    }
    free(buf);
    if (err != DRACONIC_HOST_OK) {
        host_tcp_async_settle_err(op, err);
        return;
    }
    host_tcp_async_settle_ok(op, host_tcp_async_num((int64_t)len));
}

static void host_tcp_async_on_write_ready(void *data) {
    HostTcpAsyncOp *op = (HostTcpAsyncOp *)data;
    DraconicHostError err;
#if !defined(_WIN32)
    int fd;
    ssize_t n;
#endif
    if (!op || op->settled) {
        return;
    }
#if defined(_WIN32)
    host_tcp_async_settle_err(op, DRACONIC_HOST_E_NOSYS);
    return;
#else
    fd = host_handle_tcp_conn_fd(op->h);
    if (fd < 0) {
        host_tcp_async_settle_err(op, DRACONIC_HOST_E_BADF);
        return;
    }
    while (op->write_off < op->write_len) {
        n = write(
            fd,
            op->write_data + op->write_off,
            op->write_len - op->write_off);
        if (n < 0) {
            if (errno == EAGAIN || errno == EWOULDBLOCK) {
                int64_t id = 0;
                err = draconic_rt_host_io_wait(
                    op->h,
                    DRACONIC_HOST_IO_WRITE,
                    host_tcp_async_on_write_ready,
                    op,
                    &id);
                if (err != DRACONIC_HOST_OK) {
                    host_tcp_async_settle_err(op, err);
                    return;
                }
                op->wait_id = id;
                return;
            }
            host_tcp_async_settle_err(op, host_tcp_errno_map());
            return;
        }
        op->write_off += (size_t)n;
    }
    host_tcp_async_settle_ok(op, host_tcp_async_num((int64_t)op->write_len));
#endif
}

DraconicValue *draconic_rt_host_tcp_accept_async(DraconicHostHandle listen_h) {
    DraconicValue *p = draconic_rt_promise_new();
    HostTcpAsyncOp *op;
    DraconicHostHandle conn = DRACONIC_HOST_HANDLE_INVALID;
    DraconicHostError err;
    int64_t id = 0;

    if (!p) {
        return NULL;
    }
    err = draconic_rt_host_tcp_set_nonblocking(listen_h, 1);
    if (err != DRACONIC_HOST_OK) {
        draconic_rt_promise_reject(p, host_tcp_async_num((int64_t)err));
        return p;
    }
    err = draconic_rt_host_tcp_accept(listen_h, &conn);
    if (err == DRACONIC_HOST_OK) {
        draconic_rt_promise_resolve(p, host_tcp_async_num((int64_t)conn));
        return p;
    }
    if (err != DRACONIC_HOST_E_AGAIN) {
        draconic_rt_promise_reject(p, host_tcp_async_num((int64_t)err));
        return p;
    }
    op = host_tcp_async_alloc(HOST_TCP_ASYNC_ACCEPT, listen_h, p);
    if (!op) {
        draconic_rt_promise_reject(p, host_tcp_async_num((int64_t)DRACONIC_HOST_E_NOMEM));
        return p;
    }
    err = draconic_rt_host_io_wait(
        listen_h, DRACONIC_HOST_IO_READ, host_tcp_async_on_accept_ready, op, &id);
    if (err != DRACONIC_HOST_OK) {
        host_tcp_async_settle_err(op, err);
        return p;
    }
    op->wait_id = id;
    return p;
}

DraconicValue *draconic_rt_host_tcp_connect_async(const char *host, int32_t port) {
    DraconicValue *p = draconic_rt_promise_new();
#if defined(_WIN32)
    if (p) {
        draconic_rt_promise_reject(p, host_tcp_async_num((int64_t)DRACONIC_HOST_E_NOSYS));
    }
    return p;
#else
    HostTcpAsyncOp *op;
    DraconicHostHandle conn = DRACONIC_HOST_HANDLE_INVALID;
    DraconicHostError err;
    int fd = -1;
    struct sockaddr_in addr;
    int64_t id = 0;
    int flags;

    if (!p) {
        return NULL;
    }
    if (!host || host[0] == '\0' || port < 1 || port > 65535) {
        draconic_rt_promise_reject(p, host_tcp_async_num((int64_t)DRACONIC_HOST_E_INVAL));
        return p;
    }

    memset(&addr, 0, sizeof(addr));
    addr.sin_family = AF_INET;
    addr.sin_port = htons((uint16_t)port);
    err = host_resolve_ipv4(host, &addr.sin_addr);
    if (err != DRACONIC_HOST_OK) {
        draconic_rt_promise_reject(p, host_tcp_async_num((int64_t)err));
        return p;
    }

    fd = (int)socket(AF_INET, SOCK_STREAM, 0);
    if (fd < 0) {
        draconic_rt_promise_reject(p, host_tcp_async_num((int64_t)host_tcp_errno_map()));
        return p;
    }
    flags = fcntl(fd, F_GETFL, 0);
    if (flags < 0 || fcntl(fd, F_SETFL, flags | O_NONBLOCK) < 0) {
        err = host_tcp_errno_map();
        (void)close(fd);
        draconic_rt_promise_reject(p, host_tcp_async_num((int64_t)err));
        return p;
    }
    if (connect(fd, (struct sockaddr *)&addr, sizeof(addr)) < 0) {
        if (errno != EINPROGRESS && errno != EAGAIN && errno != EWOULDBLOCK) {
            err = host_tcp_errno_map();
            (void)close(fd);
            draconic_rt_promise_reject(p, host_tcp_async_num((int64_t)err));
            return p;
        }
        err = host_handle_alloc(DRACONIC_HOST_HANDLE_KIND_TCP_CONN, fd, &conn);
        if (err != DRACONIC_HOST_OK) {
            (void)close(fd);
            draconic_rt_promise_reject(p, host_tcp_async_num((int64_t)err));
            return p;
        }
        op = host_tcp_async_alloc(HOST_TCP_ASYNC_CONNECT, conn, p);
        if (!op) {
            (void)draconic_rt_host_handle_close(conn);
            draconic_rt_promise_reject(p, host_tcp_async_num((int64_t)DRACONIC_HOST_E_NOMEM));
            return p;
        }
        op->connect_host = strdup(host);
        op->connect_port = port;
        err = draconic_rt_host_io_wait(
            conn, DRACONIC_HOST_IO_WRITE, host_tcp_async_on_connect_ready, op, &id);
        if (err != DRACONIC_HOST_OK) {
            host_tcp_async_settle_err(op, err);
            (void)draconic_rt_host_handle_close(conn);
            return p;
        }
        op->wait_id = id;
        return p;
    }
    /* Immediate connect success. */
    err = host_handle_alloc(DRACONIC_HOST_HANDLE_KIND_TCP_CONN, fd, &conn);
    if (err != DRACONIC_HOST_OK) {
        (void)close(fd);
        draconic_rt_promise_reject(p, host_tcp_async_num((int64_t)err));
        return p;
    }
    draconic_rt_promise_resolve(p, host_tcp_async_num((int64_t)conn));
    return p;
#endif
}

DraconicValue *draconic_rt_host_tcp_read_async(
    DraconicHostHandle conn_h,
    int64_t max_len) {
    DraconicValue *p = draconic_rt_promise_new();
    HostTcpAsyncOp *op;
    uint8_t *buf = NULL;
    size_t len = 0;
    DraconicHostError err;
    int64_t id = 0;

    if (!p) {
        return NULL;
    }
    if (max_len < 0) {
        draconic_rt_promise_reject(p, host_tcp_async_num((int64_t)DRACONIC_HOST_E_INVAL));
        return p;
    }
    err = draconic_rt_host_tcp_set_nonblocking(conn_h, 1);
    if (err != DRACONIC_HOST_OK) {
        draconic_rt_promise_reject(p, host_tcp_async_num((int64_t)err));
        return p;
    }
    err = draconic_rt_host_tcp_read(conn_h, (size_t)max_len, &buf, &len);
    if (err == DRACONIC_HOST_OK) {
        free(buf);
        draconic_rt_promise_resolve(p, host_tcp_async_num((int64_t)len));
        return p;
    }
    if (err != DRACONIC_HOST_E_AGAIN) {
        draconic_rt_promise_reject(p, host_tcp_async_num((int64_t)err));
        return p;
    }
    op = host_tcp_async_alloc(HOST_TCP_ASYNC_READ, conn_h, p);
    if (!op) {
        draconic_rt_promise_reject(p, host_tcp_async_num((int64_t)DRACONIC_HOST_E_NOMEM));
        return p;
    }
    op->max_len = max_len;
    err = draconic_rt_host_io_wait(
        conn_h, DRACONIC_HOST_IO_READ, host_tcp_async_on_read_ready, op, &id);
    if (err != DRACONIC_HOST_OK) {
        host_tcp_async_settle_err(op, err);
        return p;
    }
    op->wait_id = id;
    return p;
}

DraconicValue *draconic_rt_host_tcp_write_async(
    DraconicHostHandle conn_h,
    const uint8_t *data,
    size_t len) {
    DraconicValue *p = draconic_rt_promise_new();
    HostTcpAsyncOp *op;
    DraconicHostError err;
    int64_t id = 0;
#if !defined(_WIN32)
    int fd;
    ssize_t n;
    size_t off = 0;
#endif

    if (!p) {
        return NULL;
    }
    if (len > 0 && !data) {
        draconic_rt_promise_reject(p, host_tcp_async_num((int64_t)DRACONIC_HOST_E_INVAL));
        return p;
    }
    err = draconic_rt_host_tcp_set_nonblocking(conn_h, 1);
    if (err != DRACONIC_HOST_OK) {
        draconic_rt_promise_reject(p, host_tcp_async_num((int64_t)err));
        return p;
    }
#if defined(_WIN32)
    draconic_rt_promise_reject(p, host_tcp_async_num((int64_t)DRACONIC_HOST_E_NOSYS));
    return p;
#else
    fd = host_handle_tcp_conn_fd(conn_h);
    if (fd < 0) {
        draconic_rt_promise_reject(p, host_tcp_async_num((int64_t)DRACONIC_HOST_E_BADF));
        return p;
    }
    while (off < len) {
        n = write(fd, data + off, len - off);
        if (n < 0) {
            if (errno == EAGAIN || errno == EWOULDBLOCK) {
                break;
            }
            draconic_rt_promise_reject(p, host_tcp_async_num((int64_t)host_tcp_errno_map()));
            return p;
        }
        off += (size_t)n;
    }
    if (off >= len) {
        draconic_rt_promise_resolve(p, host_tcp_async_num((int64_t)len));
        return p;
    }
    op = host_tcp_async_alloc(HOST_TCP_ASYNC_WRITE, conn_h, p);
    if (!op) {
        draconic_rt_promise_reject(p, host_tcp_async_num((int64_t)DRACONIC_HOST_E_NOMEM));
        return p;
    }
    op->write_len = len;
    op->write_off = off;
    op->write_data = (uint8_t *)malloc(len);
    if (!op->write_data) {
        host_tcp_async_settle_err(op, DRACONIC_HOST_E_NOMEM);
        return p;
    }
    memcpy(op->write_data, data, len);
    err = draconic_rt_host_io_wait(
        conn_h, DRACONIC_HOST_IO_WRITE, host_tcp_async_on_write_ready, op, &id);
    if (err != DRACONIC_HOST_OK) {
        host_tcp_async_settle_err(op, err);
        return p;
    }
    op->wait_id = id;
    return p;
#endif
}

/* --- UDP bind/sendto/recvfrom (H08.01) ----------------------------------- */

#if !defined(_WIN32)
static int host_handle_udp_fd(DraconicHostHandle h) {
    if (!draconic_rt_host_handle_is_valid(h)) {
        return -1;
    }
    if (g_host_handle_kind[(size_t)h - 1] != DRACONIC_HOST_HANDLE_KIND_UDP) {
        return -1;
    }
    return g_host_handle_fd[(size_t)h - 1];
}
#endif

DraconicHostError draconic_rt_host_udp_bind(
    int32_t port,
    DraconicHostHandle *out_h) {
#if defined(_WIN32)
    (void)port;
    (void)out_h;
    return DRACONIC_HOST_E_NOSYS;
#else
    int fd = -1;
    int yes = 1;
    struct sockaddr_in addr;
    DraconicHostError err;

    if (!out_h) {
        return DRACONIC_HOST_E_INVAL;
    }
    *out_h = DRACONIC_HOST_HANDLE_INVALID;
    if (port < 0 || port > 65535) {
        return DRACONIC_HOST_E_INVAL;
    }

    fd = (int)socket(AF_INET, SOCK_DGRAM, 0);
    if (fd < 0) {
        return host_tcp_errno_map();
    }

    if (setsockopt(fd, SOL_SOCKET, SO_REUSEADDR, &yes, sizeof(yes)) < 0) {
        err = host_tcp_errno_map();
        (void)close(fd);
        return err;
    }

    memset(&addr, 0, sizeof(addr));
    addr.sin_family = AF_INET;
    addr.sin_addr.s_addr = htonl(INADDR_ANY);
    addr.sin_port = htons((uint16_t)port);

    if (bind(fd, (struct sockaddr *)&addr, sizeof(addr)) < 0) {
        err = host_tcp_errno_map();
        (void)close(fd);
        return err;
    }

    err = host_handle_alloc(DRACONIC_HOST_HANDLE_KIND_UDP, fd, out_h);
    if (err != DRACONIC_HOST_OK) {
        (void)close(fd);
        return err;
    }
    return DRACONIC_HOST_OK;
#endif
}

DraconicHostError draconic_rt_host_udp_local_port(
    DraconicHostHandle h,
    int32_t *out_port) {
#if defined(_WIN32)
    (void)h;
    (void)out_port;
    return DRACONIC_HOST_E_NOSYS;
#else
    int fd;
    struct sockaddr_in addr;
    socklen_t len = (socklen_t)sizeof(addr);

    if (!out_port) {
        return DRACONIC_HOST_E_INVAL;
    }
    *out_port = 0;
    fd = host_handle_udp_fd(h);
    if (fd < 0) {
        return DRACONIC_HOST_E_BADF;
    }
    memset(&addr, 0, sizeof(addr));
    if (getsockname(fd, (struct sockaddr *)&addr, &len) < 0) {
        return host_tcp_errno_map();
    }
    *out_port = (int32_t)ntohs(addr.sin_port);
    return DRACONIC_HOST_OK;
#endif
}

DraconicHostError draconic_rt_host_udp_sendto(
    DraconicHostHandle h,
    const uint8_t *data,
    size_t len,
    const char *host,
    int32_t port) {
#if defined(_WIN32)
    (void)h;
    (void)data;
    (void)len;
    (void)host;
    (void)port;
    return DRACONIC_HOST_E_NOSYS;
#else
    int fd;
    struct sockaddr_in addr;
    ssize_t n;

    fd = host_handle_udp_fd(h);
    if (fd < 0) {
        return DRACONIC_HOST_E_BADF;
    }
    if (!host || port < 1 || port > 65535) {
        return DRACONIC_HOST_E_INVAL;
    }
    if (len == 0) {
        return DRACONIC_HOST_OK;
    }
    if (!data) {
        return DRACONIC_HOST_E_INVAL;
    }

    memset(&addr, 0, sizeof(addr));
    addr.sin_family = AF_INET;
    addr.sin_port = htons((uint16_t)port);
    if (inet_pton(AF_INET, host, &addr.sin_addr) != 1) {
        return DRACONIC_HOST_E_INVAL;
    }

    n = sendto(fd, data, len, 0, (struct sockaddr *)&addr, sizeof(addr));
    if (n < 0) {
        return host_tcp_errno_map();
    }
    if ((size_t)n != len) {
        return DRACONIC_HOST_E_IO;
    }
    return DRACONIC_HOST_OK;
#endif
}

DraconicHostError draconic_rt_host_udp_recvfrom(
    DraconicHostHandle h,
    size_t max_len,
    uint8_t **out_data,
    size_t *out_len,
    char **out_peer_addr,
    int32_t *out_peer_port) {
#if defined(_WIN32)
    (void)h;
    (void)max_len;
    (void)out_data;
    (void)out_len;
    (void)out_peer_addr;
    (void)out_peer_port;
    return DRACONIC_HOST_E_NOSYS;
#else
    int fd;
    uint8_t *buf = NULL;
    struct sockaddr_in peer;
    socklen_t plen = (socklen_t)sizeof(peer);
    ssize_t n;
    size_t got;

    if (!out_data || !out_len) {
        return DRACONIC_HOST_E_INVAL;
    }
    *out_data = NULL;
    *out_len = 0;
    if (out_peer_addr) {
        *out_peer_addr = NULL;
    }
    if (out_peer_port) {
        *out_peer_port = 0;
    }

    fd = host_handle_udp_fd(h);
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
    memset(&peer, 0, sizeof(peer));
    n = recvfrom(fd, buf, max_len, 0, (struct sockaddr *)&peer, &plen);
    if (n < 0) {
        free(buf);
        return host_tcp_errno_map();
    }
    got = (size_t)n;
    if (got == 0) {
        free(buf);
        *out_data = NULL;
        *out_len = 0;
    } else {
        if (got < max_len) {
            uint8_t *shrunk = (uint8_t *)realloc(buf, got);
            if (shrunk) {
                buf = shrunk;
            }
        }
        *out_data = buf;
        *out_len = got;
    }

    if (out_peer_port) {
        *out_peer_port = (int32_t)ntohs(peer.sin_port);
    }
    if (out_peer_addr) {
        char abuf[INET_ADDRSTRLEN];
        char *dup;
        if (!inet_ntop(AF_INET, &peer.sin_addr, abuf, sizeof(abuf))) {
            if (*out_data) {
                free(*out_data);
                *out_data = NULL;
                *out_len = 0;
            }
            return host_tcp_errno_map();
        }
        dup = (char *)malloc(strlen(abuf) + 1);
        if (!dup) {
            if (*out_data) {
                free(*out_data);
                *out_data = NULL;
                *out_len = 0;
            }
            return DRACONIC_HOST_E_NOMEM;
        }
        memcpy(dup, abuf, strlen(abuf) + 1);
        *out_peer_addr = dup;
    }
    return DRACONIC_HOST_OK;
#endif
}

/* --- DNS lookup (H09.01) -------------------------------------------------- */

static void host_dns_free_addrs(char **addrs, size_t count) {
    size_t i;
    if (!addrs) {
        return;
    }
    for (i = 0; i < count; i++) {
        free(addrs[i]);
    }
    free(addrs);
}

static int host_dns_already_has(char **addrs, size_t count, const char *s) {
    size_t i;
    for (i = 0; i < count; i++) {
        if (addrs[i] && strcmp(addrs[i], s) == 0) {
            return 1;
        }
    }
    return 0;
}

DraconicHostError draconic_rt_host_dns_lookup(
    const char *hostname,
    char ***out_addrs,
    int64_t *out_count) {
#if defined(_WIN32)
    (void)hostname;
    (void)out_addrs;
    (void)out_count;
    return DRACONIC_HOST_E_NOSYS;
#else
    struct addrinfo hints;
    struct addrinfo *res = NULL;
    struct addrinfo *rp;
    char **addrs = NULL;
    size_t count = 0;
    size_t cap = 0;
    int gai;

    if (!hostname || hostname[0] == '\0' || !out_addrs || !out_count) {
        return DRACONIC_HOST_E_INVAL;
    }
    *out_addrs = NULL;
    *out_count = 0;

    memset(&hints, 0, sizeof(hints));
    hints.ai_family = AF_INET;
    hints.ai_socktype = SOCK_STREAM;

    gai = getaddrinfo(hostname, NULL, &hints, &res);
    if (gai != 0) {
        return DRACONIC_HOST_E_ADDR;
    }

    for (rp = res; rp != NULL; rp = rp->ai_next) {
        char abuf[INET_ADDRSTRLEN];
        char *copy;
        char **grown;
        struct sockaddr_in *sa;

        if (rp->ai_family != AF_INET || !rp->ai_addr) {
            continue;
        }
        sa = (struct sockaddr_in *)rp->ai_addr;
        if (!inet_ntop(AF_INET, &sa->sin_addr, abuf, sizeof(abuf))) {
            continue;
        }
        if (host_dns_already_has(addrs, count, abuf)) {
            continue;
        }
        if (count == cap) {
            size_t ncap = cap == 0 ? 4 : cap * 2;
            grown = (char **)realloc(addrs, ncap * sizeof(char *));
            if (!grown) {
                freeaddrinfo(res);
                host_dns_free_addrs(addrs, count);
                return DRACONIC_HOST_E_NOMEM;
            }
            addrs = grown;
            cap = ncap;
        }
        copy = (char *)malloc(strlen(abuf) + 1);
        if (!copy) {
            freeaddrinfo(res);
            host_dns_free_addrs(addrs, count);
            return DRACONIC_HOST_E_NOMEM;
        }
        memcpy(copy, abuf, strlen(abuf) + 1);
        addrs[count++] = copy;
    }
    freeaddrinfo(res);

    if (count == 0) {
        host_dns_free_addrs(addrs, count);
        return DRACONIC_HOST_E_ADDR;
    }

    *out_addrs = addrs;
    *out_count = (int64_t)count;
    return DRACONIC_HOST_OK;
#endif
}

/* --- HTTP/1.1 request parse (H10.01) -------------------------------------- */

static char *host_http_dup_range(const uint8_t *p, size_t n) {
    char *s;
    s = (char *)malloc(n + 1);
    if (!s) {
        return NULL;
    }
    if (n > 0) {
        memcpy(s, p, n);
    }
    s[n] = '\0';
    return s;
}

static char *host_http_dup_cstr(const char *s) {
    size_t n;
    if (!s) {
        return host_http_dup_range(NULL, 0);
    }
    n = strlen(s);
    return host_http_dup_range((const uint8_t *)s, n);
}

/* Find CRLFCRLF terminator; returns index of first byte of body, or (size_t)-1. */
static size_t host_http_find_header_end(const uint8_t *data, size_t len) {
    size_t i;
    if (len < 4) {
        return (size_t)-1;
    }
    for (i = 0; i + 3 < len; i++) {
        if (data[i] == '\r' && data[i + 1] == '\n'
            && data[i + 2] == '\r' && data[i + 3] == '\n') {
            return i + 4;
        }
    }
    return (size_t)-1;
}

static int host_http_ascii_ieq(const char *a, size_t alen, const char *b) {
    size_t i;
    size_t blen = strlen(b);
    if (alen != blen) {
        return 0;
    }
    for (i = 0; i < alen; i++) {
        unsigned char ca = (unsigned char)a[i];
        unsigned char cb = (unsigned char)b[i];
        if (ca >= 'A' && ca <= 'Z') {
            ca = (unsigned char)(ca - 'A' + 'a');
        }
        if (cb >= 'A' && cb <= 'Z') {
            cb = (unsigned char)(cb - 'A' + 'a');
        }
        if (ca != cb) {
            return 0;
        }
    }
    return 1;
}

/* Parse Content-Length value (non-negative decimal). Returns 0 on OK. */
static int host_http_parse_content_length(const char *v, size_t vlen, size_t *out_cl) {
    size_t i;
    size_t n = 0;
    if (vlen == 0) {
        return -1;
    }
    for (i = 0; i < vlen; i++) {
        unsigned char c = (unsigned char)v[i];
        if (c < '0' || c > '9') {
            return -1;
        }
        n = n * 10 + (size_t)(c - '0');
    }
    *out_cl = n;
    return 0;
}

/* Walk headers [hdr_start, hdr_end) for name; *out_val/*out_vlen set if found. */
static int host_http_find_header(
    const uint8_t *data,
    size_t hdr_start,
    size_t hdr_end,
    const char *name,
    const char **out_val,
    size_t *out_vlen) {
    size_t i = hdr_start;
    while (i < hdr_end) {
        size_t line_end = i;
        size_t colon;
        size_t name_end;
        size_t vstart;
        size_t vend;
        while (line_end + 1 < hdr_end
            && !(data[line_end] == '\r' && data[line_end + 1] == '\n')) {
            line_end++;
        }
        if (line_end + 1 >= hdr_end) {
            break;
        }
        /* empty line should not appear before header_end */
        if (line_end == i) {
            i = line_end + 2;
            continue;
        }
        colon = i;
        while (colon < line_end && data[colon] != ':') {
            colon++;
        }
        if (colon >= line_end) {
            i = line_end + 2;
            continue;
        }
        name_end = colon;
        while (name_end > i
            && (data[name_end - 1] == ' ' || data[name_end - 1] == '\t')) {
            name_end--;
        }
        if (host_http_ascii_ieq((const char *)(data + i), name_end - i, name)) {
            vstart = colon + 1;
            while (vstart < line_end
                && (data[vstart] == ' ' || data[vstart] == '\t')) {
                vstart++;
            }
            vend = line_end;
            while (vend > vstart
                && (data[vend - 1] == ' ' || data[vend - 1] == '\t')) {
                vend--;
            }
            *out_val = (const char *)(data + vstart);
            *out_vlen = vend - vstart;
            return 1;
        }
        i = line_end + 2;
    }
    return 0;
}

DraconicHostError draconic_rt_host_http_parse_request(
    const uint8_t *data,
    size_t len,
    char **out_method,
    char **out_path,
    char **out_version,
    char **out_body) {
    size_t body_off;
    size_t i;
    size_t sp1;
    size_t sp2;
    size_t line_end;
    size_t cl;
    int has_cl;
    const char *hv;
    size_t hvlen;
    char *method = NULL;
    char *path = NULL;
    char *version = NULL;
    char *body = NULL;
    size_t body_len;

    if (!data || !out_method || !out_path || !out_version || !out_body) {
        return DRACONIC_HOST_E_INVAL;
    }
    *out_method = NULL;
    *out_path = NULL;
    *out_version = NULL;
    *out_body = NULL;

    body_off = host_http_find_header_end(data, len);
    if (body_off == (size_t)-1) {
        return DRACONIC_HOST_E_INVAL;
    }

    /* request-line: METHOD SP path SP version CRLF */
    line_end = 0;
    while (line_end + 1 < body_off
        && !(data[line_end] == '\r' && data[line_end + 1] == '\n')) {
        line_end++;
    }
    if (line_end + 1 >= body_off || line_end == 0) {
        return DRACONIC_HOST_E_INVAL;
    }

    sp1 = 0;
    while (sp1 < line_end && data[sp1] != ' ') {
        sp1++;
    }
    if (sp1 == 0 || sp1 >= line_end) {
        return DRACONIC_HOST_E_INVAL;
    }
    sp2 = sp1 + 1;
    while (sp2 < line_end && data[sp2] != ' ') {
        sp2++;
    }
    if (sp2 <= sp1 + 1 || sp2 >= line_end) {
        return DRACONIC_HOST_E_INVAL;
    }
    /* no extra spaces in method/path; version is rest of line */
    for (i = 0; i < sp1; i++) {
        if (data[i] == ' ' || data[i] == '\t') {
            return DRACONIC_HOST_E_INVAL;
        }
    }

    method = host_http_dup_range(data, sp1);
    path = host_http_dup_range(data + sp1 + 1, sp2 - (sp1 + 1));
    version = host_http_dup_range(data + sp2 + 1, line_end - (sp2 + 1));
    if (!method || !path || !version) {
        free(method);
        free(path);
        free(version);
        return DRACONIC_HOST_E_NOMEM;
    }

    /* Content-Length bounds body; headers start after request-line CRLF. */
    has_cl = host_http_find_header(
        data, line_end + 2, body_off - 2, "Content-Length", &hv, &hvlen);
    body_len = 0;
    if (has_cl) {
        if (host_http_parse_content_length(hv, hvlen, &cl) != 0) {
            free(method);
            free(path);
            free(version);
            return DRACONIC_HOST_E_INVAL;
        }
        if (body_off + cl <= len) {
            body_len = cl;
        } else {
            body_len = len - body_off;
        }
    }

    body = host_http_dup_range(data + body_off, body_len);
    if (!body) {
        free(method);
        free(path);
        free(version);
        return DRACONIC_HOST_E_NOMEM;
    }

    *out_method = method;
    *out_path = path;
    *out_version = version;
    *out_body = body;
    return DRACONIC_HOST_OK;
}

DraconicHostError draconic_rt_host_http_request_header(
    const uint8_t *data,
    size_t len,
    const char *name,
    char **out_value) {
    size_t body_off;
    size_t line_end;
    const char *hv;
    size_t hvlen;
    char *dup;

    if (!data || !name || name[0] == '\0' || !out_value) {
        return DRACONIC_HOST_E_INVAL;
    }
    *out_value = NULL;

    body_off = host_http_find_header_end(data, len);
    if (body_off == (size_t)-1) {
        return DRACONIC_HOST_E_INVAL;
    }

    line_end = 0;
    while (line_end + 1 < body_off
        && !(data[line_end] == '\r' && data[line_end + 1] == '\n')) {
        line_end++;
    }
    if (line_end + 1 >= body_off) {
        return DRACONIC_HOST_E_INVAL;
    }

    if (host_http_find_header(
            data, line_end + 2, body_off - 2, name, &hv, &hvlen)) {
        dup = host_http_dup_range((const uint8_t *)hv, hvlen);
    } else {
        dup = host_http_dup_cstr("");
    }
    if (!dup) {
        return DRACONIC_HOST_E_NOMEM;
    }
    *out_value = dup;
    return DRACONIC_HOST_OK;
}

/* --- HTTP/1.1 response write (H10.02) ------------------------------------- */

static const char *host_http_default_reason(int32_t status) {
    switch (status) {
    case 200:
        return "OK";
    case 201:
        return "Created";
    case 204:
        return "No Content";
    case 301:
        return "Moved Permanently";
    case 302:
        return "Found";
    case 304:
        return "Not Modified";
    case 400:
        return "Bad Request";
    case 401:
        return "Unauthorized";
    case 403:
        return "Forbidden";
    case 404:
        return "Not Found";
    case 405:
        return "Method Not Allowed";
    case 500:
        return "Internal Server Error";
    case 502:
        return "Bad Gateway";
    case 503:
        return "Service Unavailable";
    default:
        return "";
    }
}

/* True if headers block already has Content-Length (case-insensitive). */
static int host_http_headers_have_content_length(const char *headers) {
    const char *p;
    size_t n;
    if (!headers || headers[0] == '\0') {
        return 0;
    }
    n = strlen(headers);
    p = headers;
    while ((size_t)(p - headers) < n) {
        const char *line = p;
        const char *eol = p;
        size_t line_len;
        while ((size_t)(eol - headers) < n
            && !(eol[0] == '\r' && (size_t)(eol - headers) + 1 < n && eol[1] == '\n')
            && *eol != '\n') {
            eol++;
        }
        line_len = (size_t)(eol - line);
        if (line_len >= 14
            && host_http_ascii_ieq(line, 14, "Content-Length")) {
            if (line_len == 14 || line[14] == ':' || line[14] == ' ' || line[14] == '\t') {
                return 1;
            }
        }
        if ((size_t)(eol - headers) + 1 < n && eol[0] == '\r' && eol[1] == '\n') {
            p = eol + 2;
        } else if ((size_t)(eol - headers) < n && *eol == '\n') {
            p = eol + 1;
        } else {
            break;
        }
    }
    return 0;
}

DraconicHostError draconic_rt_host_http_write_response(
    int32_t status,
    const char *reason,
    const char *headers,
    const uint8_t *body,
    size_t body_len,
    char **out_msg) {
    const char *r;
    const char *hdrs;
    size_t hdrs_len;
    int need_cl;
    char status_buf[16];
    char cl_buf[64];
    size_t status_len;
    size_t reason_len;
    size_t cl_len;
    size_t total;
    char *msg;
    size_t off;
    int n;

    if (!out_msg) {
        return DRACONIC_HOST_E_INVAL;
    }
    *out_msg = NULL;

    if (status < 100 || status > 599) {
        return DRACONIC_HOST_E_INVAL;
    }
    if (body_len > 0 && !body) {
        return DRACONIC_HOST_E_INVAL;
    }

    if (reason && reason[0] != '\0') {
        r = reason;
    } else {
        r = host_http_default_reason(status);
    }
    reason_len = strlen(r);

    hdrs = headers ? headers : "";
    hdrs_len = strlen(hdrs);
    /* Ensure header block ends with CRLF when non-empty and missing terminator. */
    need_cl = !host_http_headers_have_content_length(hdrs);

    n = snprintf(status_buf, sizeof(status_buf), "%d", (int)status);
    if (n < 0 || (size_t)n >= sizeof(status_buf)) {
        return DRACONIC_HOST_E_INVAL;
    }
    status_len = (size_t)n;

    cl_len = 0;
    if (need_cl) {
        n = snprintf(
            cl_buf,
            sizeof(cl_buf),
            "Content-Length: %llu\r\n",
            (unsigned long long)body_len);
        if (n < 0 || (size_t)n >= sizeof(cl_buf)) {
            return DRACONIC_HOST_E_INVAL;
        }
        cl_len = (size_t)n;
    }

    /* "HTTP/1.1 " + status + " " + reason + "\r\n" + headers + [?CRLF] + cl + "\r\n" + body */
    total = 9 + status_len + 1 + reason_len + 2 + hdrs_len;
    if (hdrs_len > 0) {
        int ends_crlf = hdrs_len >= 2
            && hdrs[hdrs_len - 2] == '\r'
            && hdrs[hdrs_len - 1] == '\n';
        if (!ends_crlf) {
            total += 2; /* append CRLF */
        }
    }
    total += cl_len + 2 + body_len;

    msg = (char *)malloc(total + 1);
    if (!msg) {
        return DRACONIC_HOST_E_NOMEM;
    }

    off = 0;
    memcpy(msg + off, "HTTP/1.1 ", 9);
    off += 9;
    memcpy(msg + off, status_buf, status_len);
    off += status_len;
    msg[off++] = ' ';
    if (reason_len > 0) {
        memcpy(msg + off, r, reason_len);
        off += reason_len;
    }
    msg[off++] = '\r';
    msg[off++] = '\n';

    if (hdrs_len > 0) {
        memcpy(msg + off, hdrs, hdrs_len);
        off += hdrs_len;
        if (!(hdrs_len >= 2 && hdrs[hdrs_len - 2] == '\r' && hdrs[hdrs_len - 1] == '\n')) {
            msg[off++] = '\r';
            msg[off++] = '\n';
        }
    }
    if (need_cl) {
        memcpy(msg + off, cl_buf, cl_len);
        off += cl_len;
    }
    msg[off++] = '\r';
    msg[off++] = '\n';
    if (body_len > 0) {
        memcpy(msg + off, body, body_len);
        off += body_len;
    }
    msg[off] = '\0';
    if (off != total) {
        free(msg);
        return DRACONIC_HOST_E_INVAL;
    }

    *out_msg = msg;
    return DRACONIC_HOST_OK;
}

/* --- HTTP/1.1 client helpers (H10.05) ------------------------------------- */

DraconicHostError draconic_rt_host_http_write_request(
    const char *method,
    const char *path,
    const char *headers,
    const uint8_t *body,
    size_t body_len,
    char **out_msg) {
    const char *m;
    const char *p;
    const char *hdrs;
    size_t method_len;
    size_t path_len;
    size_t hdrs_len;
    int need_cl;
    char cl_buf[64];
    size_t cl_len;
    size_t total;
    char *msg;
    size_t off;
    int n;
    size_t i;

    if (!out_msg) {
        return DRACONIC_HOST_E_INVAL;
    }
    *out_msg = NULL;

    if (!method || method[0] == '\0' || !path || path[0] == '\0') {
        return DRACONIC_HOST_E_INVAL;
    }
    if (body_len > 0 && !body) {
        return DRACONIC_HOST_E_INVAL;
    }

    m = method;
    p = path;
    method_len = strlen(m);
    path_len = strlen(p);
    /* method/path must not contain SP/HTAB/CR/LF */
    for (i = 0; i < method_len; i++) {
        unsigned char c = (unsigned char)m[i];
        if (c == ' ' || c == '\t' || c == '\r' || c == '\n') {
            return DRACONIC_HOST_E_INVAL;
        }
    }
    for (i = 0; i < path_len; i++) {
        unsigned char c = (unsigned char)p[i];
        if (c == ' ' || c == '\t' || c == '\r' || c == '\n') {
            return DRACONIC_HOST_E_INVAL;
        }
    }

    hdrs = headers ? headers : "";
    hdrs_len = strlen(hdrs);
    need_cl = !host_http_headers_have_content_length(hdrs);

    cl_len = 0;
    if (need_cl) {
        n = snprintf(
            cl_buf,
            sizeof(cl_buf),
            "Content-Length: %llu\r\n",
            (unsigned long long)body_len);
        if (n < 0 || (size_t)n >= sizeof(cl_buf)) {
            return DRACONIC_HOST_E_INVAL;
        }
        cl_len = (size_t)n;
    }

    /* method + " " + path + " HTTP/1.1\r\n" + headers + [?CRLF] + cl + "\r\n" + body */
    total = method_len + 1 + path_len + 11 + hdrs_len;
    if (hdrs_len > 0) {
        int ends_crlf = hdrs_len >= 2
            && hdrs[hdrs_len - 2] == '\r'
            && hdrs[hdrs_len - 1] == '\n';
        if (!ends_crlf) {
            total += 2;
        }
    }
    total += cl_len + 2 + body_len;

    msg = (char *)malloc(total + 1);
    if (!msg) {
        return DRACONIC_HOST_E_NOMEM;
    }

    off = 0;
    memcpy(msg + off, m, method_len);
    off += method_len;
    msg[off++] = ' ';
    memcpy(msg + off, p, path_len);
    off += path_len;
    memcpy(msg + off, " HTTP/1.1\r\n", 11);
    off += 11;

    if (hdrs_len > 0) {
        memcpy(msg + off, hdrs, hdrs_len);
        off += hdrs_len;
        if (!(hdrs_len >= 2 && hdrs[hdrs_len - 2] == '\r' && hdrs[hdrs_len - 1] == '\n')) {
            msg[off++] = '\r';
            msg[off++] = '\n';
        }
    }
    if (need_cl) {
        memcpy(msg + off, cl_buf, cl_len);
        off += cl_len;
    }
    msg[off++] = '\r';
    msg[off++] = '\n';
    if (body_len > 0) {
        memcpy(msg + off, body, body_len);
        off += body_len;
    }
    msg[off] = '\0';
    if (off != total) {
        free(msg);
        return DRACONIC_HOST_E_INVAL;
    }

    *out_msg = msg;
    return DRACONIC_HOST_OK;
}

DraconicHostError draconic_rt_host_http_parse_response(
    const uint8_t *data,
    size_t len,
    char **out_version,
    int32_t *out_status,
    char **out_reason,
    char **out_body) {
    size_t body_off;
    size_t line_end;
    size_t sp1;
    size_t sp2;
    size_t i;
    size_t cl;
    int has_cl;
    const char *hv;
    size_t hvlen;
    char *version = NULL;
    char *reason = NULL;
    char *body = NULL;
    size_t body_len;
    int32_t status = 0;
    size_t status_digits;

    if (!data || !out_version || !out_status || !out_reason || !out_body) {
        return DRACONIC_HOST_E_INVAL;
    }
    *out_version = NULL;
    *out_status = 0;
    *out_reason = NULL;
    *out_body = NULL;

    body_off = host_http_find_header_end(data, len);
    if (body_off == (size_t)-1) {
        return DRACONIC_HOST_E_INVAL;
    }

    /* status-line: HTTP-version SP status-code SP reason-phrase CRLF */
    line_end = 0;
    while (line_end + 1 < body_off
        && !(data[line_end] == '\r' && data[line_end + 1] == '\n')) {
        line_end++;
    }
    if (line_end + 1 >= body_off || line_end == 0) {
        return DRACONIC_HOST_E_INVAL;
    }

    /* version SP status-code [SP reason] */
    sp1 = 0;
    while (sp1 < line_end && data[sp1] != ' ') {
        sp1++;
    }
    if (sp1 == 0 || sp1 + 1 >= line_end) {
        return DRACONIC_HOST_E_INVAL;
    }
    /* three status digits immediately after first SP */
    if (sp1 + 3 > line_end) {
        return DRACONIC_HOST_E_INVAL;
    }
    status = 0;
    for (i = 0; i < 3; i++) {
        unsigned char c = data[sp1 + 1 + i];
        if (c < '0' || c > '9') {
            return DRACONIC_HOST_E_INVAL;
        }
        status = status * 10 + (int32_t)(c - '0');
    }
    if (status < 100 || status > 599) {
        return DRACONIC_HOST_E_INVAL;
    }
    status_digits = sp1 + 1 + 3; /* index just after status code */
    if (status_digits < line_end) {
        if (data[status_digits] != ' ') {
            return DRACONIC_HOST_E_INVAL;
        }
        sp2 = status_digits; /* SP before reason */
    } else {
        sp2 = line_end; /* no reason */
    }

    version = host_http_dup_range(data, sp1);
    if (sp2 < line_end) {
        reason = host_http_dup_range(data + sp2 + 1, line_end - (sp2 + 1));
    } else {
        reason = host_http_dup_cstr("");
    }
    if (!version || !reason) {
        free(version);
        free(reason);
        return DRACONIC_HOST_E_NOMEM;
    }

    has_cl = host_http_find_header(
        data, line_end + 2, body_off - 2, "Content-Length", &hv, &hvlen);
    body_len = 0;
    if (has_cl) {
        if (host_http_parse_content_length(hv, hvlen, &cl) != 0) {
            free(version);
            free(reason);
            return DRACONIC_HOST_E_INVAL;
        }
        if (body_off + cl <= len) {
            body_len = cl;
        } else {
            body_len = len - body_off;
        }
    }

    body = host_http_dup_range(data + body_off, body_len);
    if (!body) {
        free(version);
        free(reason);
        return DRACONIC_HOST_E_NOMEM;
    }

    *out_version = version;
    *out_status = status;
    *out_reason = reason;
    *out_body = body;
    return DRACONIC_HOST_OK;
}

DraconicHostError draconic_rt_host_http_response_header(
    const uint8_t *data,
    size_t len,
    const char *name,
    char **out_value) {
    /* Same wire layout as request (start-line + headers + body). */
    return draconic_rt_host_http_request_header(data, len, name, out_value);
}

/* --- WebSocket handshake response (H12.01 / RFC 6455) -------------------- */

#define HOST_WS_GUID "258EAFA5-E914-47DA-95CA-C5AB0DC85B11"

static uint32_t host_sha1_rol(uint32_t v, int n) {
    return (v << n) | (v >> (32 - n));
}

static void host_sha1(const uint8_t *data, size_t len, uint8_t out[20]) {
    uint32_t h0 = 0x67452301u;
    uint32_t h1 = 0xEFCDAB89u;
    uint32_t h2 = 0x98BADCFEu;
    uint32_t h3 = 0x10325476u;
    uint32_t h4 = 0xC3D2E1F0u;
    size_t bit_len = len * 8;
    size_t pad_len = (len + 1 + 8 + 63) / 64 * 64;
    uint8_t *msg;
    size_t i, chunk;

    msg = (uint8_t *)calloc(1, pad_len);
    if (!msg) {
        memset(out, 0, 20);
        return;
    }
    if (len > 0 && data) {
        memcpy(msg, data, len);
    }
    msg[len] = 0x80;
    msg[pad_len - 8] = (uint8_t)((bit_len >> 56) & 0xff);
    msg[pad_len - 7] = (uint8_t)((bit_len >> 48) & 0xff);
    msg[pad_len - 6] = (uint8_t)((bit_len >> 40) & 0xff);
    msg[pad_len - 5] = (uint8_t)((bit_len >> 32) & 0xff);
    msg[pad_len - 4] = (uint8_t)((bit_len >> 24) & 0xff);
    msg[pad_len - 3] = (uint8_t)((bit_len >> 16) & 0xff);
    msg[pad_len - 2] = (uint8_t)((bit_len >> 8) & 0xff);
    msg[pad_len - 1] = (uint8_t)(bit_len & 0xff);

    for (chunk = 0; chunk < pad_len; chunk += 64) {
        uint32_t w[80];
        uint32_t a, b, c, d, e;
        for (i = 0; i < 16; i++) {
            size_t o = chunk + i * 4;
            w[i] = ((uint32_t)msg[o] << 24) | ((uint32_t)msg[o + 1] << 16)
                | ((uint32_t)msg[o + 2] << 8) | (uint32_t)msg[o + 3];
        }
        for (i = 16; i < 80; i++) {
            w[i] = host_sha1_rol(w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16], 1);
        }
        a = h0;
        b = h1;
        c = h2;
        d = h3;
        e = h4;
        for (i = 0; i < 80; i++) {
            uint32_t f, k, temp;
            if (i < 20) {
                f = (b & c) | ((~b) & d);
                k = 0x5A827999u;
            } else if (i < 40) {
                f = b ^ c ^ d;
                k = 0x6ED9EBA1u;
            } else if (i < 60) {
                f = (b & c) | (b & d) | (c & d);
                k = 0x8F1BBCDCu;
            } else {
                f = b ^ c ^ d;
                k = 0xCA62C1D6u;
            }
            temp = host_sha1_rol(a, 5) + f + e + k + w[i];
            e = d;
            d = c;
            c = host_sha1_rol(b, 30);
            b = a;
            a = temp;
        }
        h0 += a;
        h1 += b;
        h2 += c;
        h3 += d;
        h4 += e;
    }
    free(msg);

    out[0] = (uint8_t)(h0 >> 24);
    out[1] = (uint8_t)(h0 >> 16);
    out[2] = (uint8_t)(h0 >> 8);
    out[3] = (uint8_t)h0;
    out[4] = (uint8_t)(h1 >> 24);
    out[5] = (uint8_t)(h1 >> 16);
    out[6] = (uint8_t)(h1 >> 8);
    out[7] = (uint8_t)h1;
    out[8] = (uint8_t)(h2 >> 24);
    out[9] = (uint8_t)(h2 >> 16);
    out[10] = (uint8_t)(h2 >> 8);
    out[11] = (uint8_t)h2;
    out[12] = (uint8_t)(h3 >> 24);
    out[13] = (uint8_t)(h3 >> 16);
    out[14] = (uint8_t)(h3 >> 8);
    out[15] = (uint8_t)h3;
    out[16] = (uint8_t)(h4 >> 24);
    out[17] = (uint8_t)(h4 >> 16);
    out[18] = (uint8_t)(h4 >> 8);
    out[19] = (uint8_t)h4;
}

static void host_b64_encode_20(const uint8_t in[20], char out[29]) {
    static const char tbl[] =
        "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    size_t i, o = 0;
    for (i = 0; i + 2 < 20; i += 3) {
        uint32_t n = ((uint32_t)in[i] << 16) | ((uint32_t)in[i + 1] << 8) | (uint32_t)in[i + 2];
        out[o++] = tbl[(n >> 18) & 63];
        out[o++] = tbl[(n >> 12) & 63];
        out[o++] = tbl[(n >> 6) & 63];
        out[o++] = tbl[n & 63];
    }
    /* 20 % 3 == 2 → one leftover pair + padding */
    {
        uint32_t n = ((uint32_t)in[18] << 16) | ((uint32_t)in[19] << 8);
        out[o++] = tbl[(n >> 18) & 63];
        out[o++] = tbl[(n >> 12) & 63];
        out[o++] = tbl[(n >> 6) & 63];
        out[o++] = '=';
    }
    out[o] = '\0';
}

DraconicHostError draconic_rt_host_ws_handshake_response(
    const char *sec_websocket_key,
    char **out_msg) {
    size_t key_len;
    size_t concat_len;
    uint8_t *concat;
    uint8_t digest[20];
    char accept[29];
    static const char prefix[] =
        "HTTP/1.1 101 Switching Protocols\r\n"
        "Upgrade: websocket\r\n"
        "Connection: Upgrade\r\n"
        "Sec-WebSocket-Accept: ";
    static const char suffix[] = "\r\n\r\n";
    size_t total;
    char *msg;
    size_t off;

    if (!out_msg) {
        return DRACONIC_HOST_E_INVAL;
    }
    *out_msg = NULL;
    if (!sec_websocket_key || sec_websocket_key[0] == '\0') {
        return DRACONIC_HOST_E_INVAL;
    }

    key_len = strlen(sec_websocket_key);
    concat_len = key_len + sizeof(HOST_WS_GUID) - 1;
    concat = (uint8_t *)malloc(concat_len);
    if (!concat) {
        return DRACONIC_HOST_E_NOMEM;
    }
    memcpy(concat, sec_websocket_key, key_len);
    memcpy(concat + key_len, HOST_WS_GUID, sizeof(HOST_WS_GUID) - 1);
    host_sha1(concat, concat_len, digest);
    free(concat);
    host_b64_encode_20(digest, accept);

    total = (sizeof(prefix) - 1) + strlen(accept) + (sizeof(suffix) - 1);
    msg = (char *)malloc(total + 1);
    if (!msg) {
        return DRACONIC_HOST_E_NOMEM;
    }
    off = 0;
    memcpy(msg + off, prefix, sizeof(prefix) - 1);
    off += sizeof(prefix) - 1;
    memcpy(msg + off, accept, strlen(accept));
    off += strlen(accept);
    memcpy(msg + off, suffix, sizeof(suffix) - 1);
    off += sizeof(suffix) - 1;
    msg[off] = '\0';
    *out_msg = msg;
    return DRACONIC_HOST_OK;
}

/* --- WebSocket frames (H12.02 / RFC 6455 §5) ------------------------------ */

static DraconicHostError host_ws_encode_frame(
    uint8_t opcode,
    const uint8_t *payload,
    size_t payload_len,
    const uint8_t *mask_key,
    uint8_t **out_data,
    size_t *out_len) {
    int masked = mask_key != NULL;
    size_t hdr;
    size_t total;
    uint8_t *buf;
    size_t off;
    size_t i;

    if (!out_data || !out_len) {
        return DRACONIC_HOST_E_INVAL;
    }
    *out_data = NULL;
    *out_len = 0;
    if (payload_len > 0 && !payload) {
        return DRACONIC_HOST_E_INVAL;
    }
    if (opcode > 0x0f) {
        return DRACONIC_HOST_E_INVAL;
    }

    hdr = 2;
    if (payload_len >= 126 && payload_len <= 0xffff) {
        hdr += 2;
    } else if (payload_len > 0xffff) {
        hdr += 8;
    }
    if (masked) {
        hdr += 4;
    }
    total = hdr + payload_len;
    buf = (uint8_t *)malloc(total ? total : 1);
    if (!buf) {
        return DRACONIC_HOST_E_NOMEM;
    }

    buf[0] = (uint8_t)(0x80u | (opcode & 0x0fu)); /* FIN=1 */
    off = 2;
    if (payload_len < 126) {
        buf[1] = (uint8_t)((masked ? 0x80u : 0u) | (uint8_t)payload_len);
    } else if (payload_len <= 0xffff) {
        buf[1] = (uint8_t)((masked ? 0x80u : 0u) | 126u);
        buf[2] = (uint8_t)((payload_len >> 8) & 0xff);
        buf[3] = (uint8_t)(payload_len & 0xff);
        off = 4;
    } else {
        buf[1] = (uint8_t)((masked ? 0x80u : 0u) | 127u);
        buf[2] = 0;
        buf[3] = 0;
        buf[4] = 0;
        buf[5] = 0;
        buf[6] = (uint8_t)((payload_len >> 24) & 0xff);
        buf[7] = (uint8_t)((payload_len >> 16) & 0xff);
        buf[8] = (uint8_t)((payload_len >> 8) & 0xff);
        buf[9] = (uint8_t)(payload_len & 0xff);
        off = 10;
    }
    if (masked) {
        memcpy(buf + off, mask_key, 4);
        off += 4;
        for (i = 0; i < payload_len; i++) {
            buf[off + i] = (uint8_t)(payload[i] ^ mask_key[i % 4]);
        }
    } else if (payload_len > 0) {
        memcpy(buf + off, payload, payload_len);
    }

    *out_data = buf;
    *out_len = total;
    return DRACONIC_HOST_OK;
}

DraconicHostError draconic_rt_host_ws_encode_text(
    const char *payload,
    uint8_t **out_data,
    size_t *out_len) {
    const uint8_t *p = (const uint8_t *)(payload ? payload : "");
    size_t n = payload ? strlen(payload) : 0;
    return host_ws_encode_frame(0x1, p, n, NULL, out_data, out_len);
}

DraconicHostError draconic_rt_host_ws_encode_binary(
    const uint8_t *payload,
    size_t payload_len,
    uint8_t **out_data,
    size_t *out_len) {
    return host_ws_encode_frame(0x2, payload, payload_len, NULL, out_data, out_len);
}

DraconicHostError draconic_rt_host_ws_encode_close(
    int32_t code,
    const char *reason,
    uint8_t **out_data,
    size_t *out_len) {
    size_t rlen = reason ? strlen(reason) : 0;
    size_t plen;
    uint8_t *payload;
    DraconicHostError err;

    if (code < 0 || code > 0xffff) {
        return DRACONIC_HOST_E_INVAL;
    }
    plen = 2 + rlen;
    payload = (uint8_t *)malloc(plen);
    if (!payload) {
        return DRACONIC_HOST_E_NOMEM;
    }
    payload[0] = (uint8_t)((code >> 8) & 0xff);
    payload[1] = (uint8_t)(code & 0xff);
    if (rlen > 0) {
        memcpy(payload + 2, reason, rlen);
    }
    err = host_ws_encode_frame(0x8, payload, plen, NULL, out_data, out_len);
    free(payload);
    return err;
}

DraconicHostError draconic_rt_host_ws_encode_ping(
    const char *payload,
    uint8_t **out_data,
    size_t *out_len) {
    const uint8_t *p = (const uint8_t *)(payload ? payload : "");
    size_t n = payload ? strlen(payload) : 0;
    if (n > 125) {
        return DRACONIC_HOST_E_INVAL;
    }
    return host_ws_encode_frame(0x9, p, n, NULL, out_data, out_len);
}

DraconicHostError draconic_rt_host_ws_encode_pong(
    const char *payload,
    uint8_t **out_data,
    size_t *out_len) {
    const uint8_t *p = (const uint8_t *)(payload ? payload : "");
    size_t n = payload ? strlen(payload) : 0;
    if (n > 125) {
        return DRACONIC_HOST_E_INVAL;
    }
    return host_ws_encode_frame(0xA, p, n, NULL, out_data, out_len);
}

DraconicHostError draconic_rt_host_ws_decode_frame(
    const uint8_t *data,
    size_t len,
    int32_t *out_fin,
    int32_t *out_opcode,
    uint8_t **out_payload,
    size_t *out_payload_len,
    int32_t *out_close_code) {
    size_t off;
    int fin;
    int opcode;
    int masked;
    uint64_t plen;
    uint8_t mask[4];
    uint8_t *payload;
    size_t i;

    if (!out_fin || !out_opcode || !out_payload || !out_payload_len || !out_close_code) {
        return DRACONIC_HOST_E_INVAL;
    }
    *out_fin = 0;
    *out_opcode = 0;
    *out_payload = NULL;
    *out_payload_len = 0;
    *out_close_code = -1;

    if (!data || len < 2) {
        return DRACONIC_HOST_E_INVAL;
    }

    fin = (data[0] >> 7) & 1;
    opcode = data[0] & 0x0f;
    masked = (data[1] >> 7) & 1;
    plen = (uint64_t)(data[1] & 0x7f);
    off = 2;

    if (plen == 126) {
        if (len < off + 2) {
            return DRACONIC_HOST_E_INVAL;
        }
        plen = ((uint64_t)data[off] << 8) | (uint64_t)data[off + 1];
        off += 2;
    } else if (plen == 127) {
        if (len < off + 8) {
            return DRACONIC_HOST_E_INVAL;
        }
        /* Reject non-zero high 32 bits (payload > 4GiB). */
        if (data[off] || data[off + 1] || data[off + 2] || data[off + 3]) {
            return DRACONIC_HOST_E_INVAL;
        }
        plen = ((uint64_t)data[off + 4] << 24) | ((uint64_t)data[off + 5] << 16)
            | ((uint64_t)data[off + 6] << 8) | (uint64_t)data[off + 7];
        off += 8;
    }

    if (masked) {
        if (len < off + 4) {
            return DRACONIC_HOST_E_INVAL;
        }
        memcpy(mask, data + off, 4);
        off += 4;
    }

    if (len < off + (size_t)plen) {
        return DRACONIC_HOST_E_INVAL;
    }

    if (plen > 0) {
        payload = (uint8_t *)malloc((size_t)plen);
        if (!payload) {
            return DRACONIC_HOST_E_NOMEM;
        }
        if (masked) {
            for (i = 0; i < (size_t)plen; i++) {
                payload[i] = (uint8_t)(data[off + i] ^ mask[i % 4]);
            }
        } else {
            memcpy(payload, data + off, (size_t)plen);
        }
    } else {
        payload = NULL;
    }

    *out_fin = fin;
    *out_opcode = opcode;
    *out_payload = payload;
    *out_payload_len = (size_t)plen;
    if (opcode == 0x8 && plen >= 2) {
        *out_close_code = (int32_t)(((uint32_t)payload[0] << 8) | (uint32_t)payload[1]);
        /* Expose reason only (strip 2-byte code) as payload for callers. */
        if (plen == 2) {
            free(payload);
            *out_payload = NULL;
            *out_payload_len = 0;
        } else {
            size_t rlen = (size_t)plen - 2;
            uint8_t *reason = (uint8_t *)malloc(rlen);
            if (!reason) {
                free(payload);
                *out_payload = NULL;
                *out_payload_len = 0;
                return DRACONIC_HOST_E_NOMEM;
            }
            memcpy(reason, payload + 2, rlen);
            free(payload);
            *out_payload = reason;
            *out_payload_len = rlen;
        }
    }
    return DRACONIC_HOST_OK;
}

/* --- WebSocket client dial (H12.03 / RFC 6455) --------------------------- */

static void host_ws_fill_mask(uint8_t mask[4]) {
#if defined(__APPLE__)
    arc4random_buf(mask, 4);
#elif defined(__linux__)
    {
        FILE *f = fopen("/dev/urandom", "rb");
        if (f && fread(mask, 1, 4, f) == 4) {
            fclose(f);
            return;
        }
        if (f) {
            fclose(f);
        }
        mask[0] = 0x37;
        mask[1] = 0xfa;
        mask[2] = 0x21;
        mask[3] = 0x3d;
    }
#else
    mask[0] = 0x37;
    mask[1] = 0xfa;
    mask[2] = 0x21;
    mask[3] = 0x3d;
#endif
}

static DraconicHostError host_ws_accept_for_key(
    const char *sec_websocket_key,
    char accept_out[29]) {
    size_t key_len;
    size_t concat_len;
    uint8_t *concat;
    uint8_t digest[20];

    if (!sec_websocket_key || sec_websocket_key[0] == '\0' || !accept_out) {
        return DRACONIC_HOST_E_INVAL;
    }
    key_len = strlen(sec_websocket_key);
    concat_len = key_len + sizeof(HOST_WS_GUID) - 1;
    concat = (uint8_t *)malloc(concat_len);
    if (!concat) {
        return DRACONIC_HOST_E_NOMEM;
    }
    memcpy(concat, sec_websocket_key, key_len);
    memcpy(concat + key_len, HOST_WS_GUID, sizeof(HOST_WS_GUID) - 1);
    host_sha1(concat, concat_len, digest);
    free(concat);
    host_b64_encode_20(digest, accept_out);
    return DRACONIC_HOST_OK;
}

DraconicHostError draconic_rt_host_ws_client_handshake_request(
    const char *path,
    const char *host,
    const char *sec_websocket_key,
    char **out_msg) {
    size_t path_len;
    size_t host_len;
    size_t key_len;
    size_t total;
    char *msg;
    size_t off;
    static const char p1[] = "GET ";
    static const char p2[] = " HTTP/1.1\r\nHost: ";
    static const char p3[] =
        "\r\nUpgrade: websocket\r\n"
        "Connection: Upgrade\r\n"
        "Sec-WebSocket-Key: ";
    static const char p4[] = "\r\nSec-WebSocket-Version: 13\r\n\r\n";

    if (!out_msg) {
        return DRACONIC_HOST_E_INVAL;
    }
    *out_msg = NULL;
    if (!path || path[0] == '\0' || !host || host[0] == '\0'
        || !sec_websocket_key || sec_websocket_key[0] == '\0') {
        return DRACONIC_HOST_E_INVAL;
    }
    path_len = strlen(path);
    host_len = strlen(host);
    key_len = strlen(sec_websocket_key);
    total = (sizeof(p1) - 1) + path_len + (sizeof(p2) - 1) + host_len
        + (sizeof(p3) - 1) + key_len + (sizeof(p4) - 1);
    msg = (char *)malloc(total + 1);
    if (!msg) {
        return DRACONIC_HOST_E_NOMEM;
    }
    off = 0;
    memcpy(msg + off, p1, sizeof(p1) - 1);
    off += sizeof(p1) - 1;
    memcpy(msg + off, path, path_len);
    off += path_len;
    memcpy(msg + off, p2, sizeof(p2) - 1);
    off += sizeof(p2) - 1;
    memcpy(msg + off, host, host_len);
    off += host_len;
    memcpy(msg + off, p3, sizeof(p3) - 1);
    off += sizeof(p3) - 1;
    memcpy(msg + off, sec_websocket_key, key_len);
    off += key_len;
    memcpy(msg + off, p4, sizeof(p4) - 1);
    off += sizeof(p4) - 1;
    msg[off] = '\0';
    *out_msg = msg;
    return DRACONIC_HOST_OK;
}

DraconicHostError draconic_rt_host_ws_client_check_accept(
    const uint8_t *data,
    size_t len,
    const char *sec_websocket_key) {
    char want[29];
    DraconicHostError err;
    const char *p;
    const char *end;
    const char *line;
    int saw_101 = 0;
    int saw_accept = 0;
    char *hdr_val = NULL;
    size_t i;

    if (!data || len == 0 || !sec_websocket_key || sec_websocket_key[0] == '\0') {
        return DRACONIC_HOST_E_INVAL;
    }
    err = host_ws_accept_for_key(sec_websocket_key, want);
    if (err != DRACONIC_HOST_OK) {
        return err;
    }

    /* Status line: HTTP/1.1 101 … */
    if (len < 12) {
        return DRACONIC_HOST_E_INVAL;
    }
    p = (const char *)data;
    end = p + len;
    /* Find end of first line */
    line = p;
    while (p + 1 < end && !(p[0] == '\r' && p[1] == '\n')) {
        p++;
    }
    if (p + 1 >= end) {
        return DRACONIC_HOST_E_INVAL;
    }
    /* Check for " 101 " in status line */
    {
        size_t slen = (size_t)(p - line);
        for (i = 0; i + 5 <= slen; i++) {
            if (line[i] == ' ' && line[i + 1] == '1' && line[i + 2] == '0'
                && line[i + 3] == '1' && (i + 4 == slen || line[i + 4] == ' ')) {
                saw_101 = 1;
                break;
            }
        }
    }
    if (!saw_101) {
        return DRACONIC_HOST_E_INVAL;
    }
    p += 2; /* skip CRLF */

    /* Scan headers until blank line */
    while (p < end) {
        const char *hstart = p;
        const char *colon;
        size_t nlen;
        if (p + 1 < end && p[0] == '\r' && p[1] == '\n') {
            break; /* end of headers */
        }
        while (p + 1 < end && !(p[0] == '\r' && p[1] == '\n')) {
            p++;
        }
        if (p + 1 >= end) {
            return DRACONIC_HOST_E_INVAL;
        }
        colon = hstart;
        while (colon < p && *colon != ':') {
            colon++;
        }
        if (colon >= p) {
            p += 2;
            continue;
        }
        nlen = (size_t)(colon - hstart);
        /* Case-insensitive Sec-WebSocket-Accept */
        if (nlen == 20
            && (hstart[0] == 'S' || hstart[0] == 's')
            && (hstart[1] == 'e' || hstart[1] == 'E')
            && (hstart[2] == 'c' || hstart[2] == 'C')
            && hstart[3] == '-'
            && (hstart[4] == 'W' || hstart[4] == 'w')
            && (hstart[5] == 'e' || hstart[5] == 'E')
            && (hstart[6] == 'b' || hstart[6] == 'B')
            && (hstart[7] == 'S' || hstart[7] == 's')
            && (hstart[8] == 'o' || hstart[8] == 'O')
            && (hstart[9] == 'c' || hstart[9] == 'C')
            && (hstart[10] == 'k' || hstart[10] == 'K')
            && (hstart[11] == 'e' || hstart[11] == 'E')
            && (hstart[12] == 't' || hstart[12] == 'T')
            && hstart[13] == '-'
            && (hstart[14] == 'A' || hstart[14] == 'a')
            && (hstart[15] == 'c' || hstart[15] == 'C')
            && (hstart[16] == 'c' || hstart[16] == 'C')
            && (hstart[17] == 'e' || hstart[17] == 'E')
            && (hstart[18] == 'p' || hstart[18] == 'P')
            && (hstart[19] == 't' || hstart[19] == 'T')) {
            const char *v = colon + 1;
            const char *vend = p;
            while (v < vend && (*v == ' ' || *v == '\t')) {
                v++;
            }
            while (vend > v && (vend[-1] == ' ' || vend[-1] == '\t')) {
                vend--;
            }
            {
                size_t vlen = (size_t)(vend - v);
                if (vlen == strlen(want) && memcmp(v, want, vlen) == 0) {
                    saw_accept = 1;
                } else {
                    return DRACONIC_HOST_E_INVAL;
                }
            }
            (void)hdr_val;
        }
        p += 2;
    }
    if (!saw_accept) {
        return DRACONIC_HOST_E_INVAL;
    }
    return DRACONIC_HOST_OK;
}

DraconicHostError draconic_rt_host_ws_encode_text_client(
    const char *payload,
    uint8_t **out_data,
    size_t *out_len) {
    const uint8_t *p = (const uint8_t *)(payload ? payload : "");
    size_t n = payload ? strlen(payload) : 0;
    uint8_t mask[4];
    host_ws_fill_mask(mask);
    return host_ws_encode_frame(0x1, p, n, mask, out_data, out_len);
}

/* --- TLS client wrap (H11.01) -------------------------------------------- */

#if defined(__APPLE__)
static OSStatus host_tls_io_read(SSLConnectionRef connection, void *data, size_t *dataLength) {
    int fd = (int)(intptr_t)connection;
    size_t want = *dataLength;
    ssize_t n;
    uint8_t *p = (uint8_t *)data;
    size_t got = 0;
    if (fd < 0 || !data || !dataLength) {
        return errSecParam;
    }
    while (got < want) {
        n = read(fd, p + got, want - got);
        if (n == 0) {
            *dataLength = got;
            return got == 0 ? errSSLClosedGraceful : noErr;
        }
        if (n < 0) {
            if (errno == EINTR) {
                continue;
            }
            if (errno == EAGAIN || errno == EWOULDBLOCK) {
                *dataLength = got;
                return errSSLWouldBlock;
            }
            return errSSLClosedAbort;
        }
        got += (size_t)n;
    }
    *dataLength = got;
    return noErr;
}

static OSStatus host_tls_io_write(SSLConnectionRef connection, const void *data, size_t *dataLength) {
    int fd = (int)(intptr_t)connection;
    size_t want = *dataLength;
    ssize_t n;
    const uint8_t *p = (const uint8_t *)data;
    size_t sent = 0;
    if (fd < 0 || !data || !dataLength) {
        return errSecParam;
    }
    while (sent < want) {
        n = write(fd, p + sent, want - sent);
        if (n < 0) {
            if (errno == EINTR) {
                continue;
            }
            if (errno == EAGAIN || errno == EWOULDBLOCK) {
                *dataLength = sent;
                return errSSLWouldBlock;
            }
            return errSSLClosedAbort;
        }
        if (n == 0) {
            *dataLength = sent;
            return errSSLClosedAbort;
        }
        sent += (size_t)n;
    }
    *dataLength = sent;
    return noErr;
}

static void host_tls_ctx_dispose(void *ctx) {
    SSLContextRef ssl = (SSLContextRef)ctx;
    if (ssl) {
        (void)SSLClose(ssl);
        CFRelease(ssl);
    }
}

static DraconicHostError host_tls_steal_tcp_fd(DraconicHostHandle h, int *out_fd) {
    size_t i;
    if (!out_fd) {
        return DRACONIC_HOST_E_INVAL;
    }
    *out_fd = -1;
    if (!draconic_rt_host_handle_is_valid(h)) {
        return DRACONIC_HOST_E_BADF;
    }
    i = (size_t)h - 1;
    if (g_host_handle_kind[i] != DRACONIC_HOST_HANDLE_KIND_TCP_CONN) {
        return DRACONIC_HOST_E_BADF;
    }
    host_tcp_async_cancel_handle(h);
    host_io_cancel_handle(h);
    *out_fd = g_host_handle_fd[i];
    g_host_handle_live[i] = 0;
    g_host_handle_kind[i] = DRACONIC_HOST_HANDLE_KIND_NONE;
    g_host_handle_fd[i] = -1;
    g_host_handle_tls_ctx[i] = NULL;
    return DRACONIC_HOST_OK;
}

static int host_handle_tls_fd(DraconicHostHandle h) {
    if (!draconic_rt_host_handle_is_valid(h)) {
        return -1;
    }
    if (g_host_handle_kind[(size_t)h - 1] != DRACONIC_HOST_HANDLE_KIND_TLS_CONN) {
        return -1;
    }
    return g_host_handle_fd[(size_t)h - 1];
}

static SSLContextRef host_handle_tls_ctx(DraconicHostHandle h) {
    if (!draconic_rt_host_handle_is_valid(h)) {
        return NULL;
    }
    if (g_host_handle_kind[(size_t)h - 1] != DRACONIC_HOST_HANDLE_KIND_TLS_CONN) {
        return NULL;
    }
    return (SSLContextRef)g_host_handle_tls_ctx[(size_t)h - 1];
}

DraconicHostError draconic_rt_host_tls_client_wrap(
    DraconicHostHandle tcp_conn,
    const char *server_name,
    int32_t insecure,
    DraconicHostHandle *out_tls) {
    int fd = -1;
    SSLContextRef ctx = NULL;
    OSStatus st;
    DraconicHostError err;
    DraconicHostHandle tls_h = DRACONIC_HOST_HANDLE_INVALID;
    size_t i;

    if (!out_tls) {
        return DRACONIC_HOST_E_INVAL;
    }
    *out_tls = DRACONIC_HOST_HANDLE_INVALID;
    if (!server_name) {
        server_name = "";
    }

    err = host_tls_steal_tcp_fd(tcp_conn, &fd);
    if (err != DRACONIC_HOST_OK) {
        return err;
    }

    ctx = SSLCreateContext(NULL, kSSLClientSide, kSSLStreamType);
    if (!ctx) {
        (void)close(fd);
        return DRACONIC_HOST_E_NOMEM;
    }
    st = SSLSetIOFuncs(ctx, host_tls_io_read, host_tls_io_write);
    if (st != noErr) {
        CFRelease(ctx);
        (void)close(fd);
        return DRACONIC_HOST_E_IO;
    }
    st = SSLSetConnection(ctx, (SSLConnectionRef)(intptr_t)fd);
    if (st != noErr) {
        CFRelease(ctx);
        (void)close(fd);
        return DRACONIC_HOST_E_IO;
    }
    if (server_name[0] != '\0') {
        st = SSLSetPeerDomainName(ctx, server_name, strlen(server_name));
        if (st != noErr) {
            CFRelease(ctx);
            (void)close(fd);
            return DRACONIC_HOST_E_INVAL;
        }
    }
    if (insecure) {
        /* Break on server auth so we can continue without trust evaluation. */
        st = SSLSetSessionOption(ctx, kSSLSessionOptionBreakOnServerAuth, true);
        if (st != noErr) {
            CFRelease(ctx);
            (void)close(fd);
            return DRACONIC_HOST_E_IO;
        }
    }

    /* Bound handshake so plain-TCP peers fail closed instead of hanging. */
    {
        struct timeval tv;
        tv.tv_sec = 2;
        tv.tv_usec = 0;
        (void)setsockopt(fd, SOL_SOCKET, SO_RCVTIMEO, &tv, sizeof(tv));
        (void)setsockopt(fd, SOL_SOCKET, SO_SNDTIMEO, &tv, sizeof(tv));
    }

    {
        time_t deadline = time(NULL) + 3;
        for (;;) {
            if (time(NULL) > deadline) {
                CFRelease(ctx);
                (void)close(fd);
                return DRACONIC_HOST_E_IO;
            }
            st = SSLHandshake(ctx);
            if (st == noErr) {
                break;
            }
            if (st == errSSLServerAuthCompleted && insecure) {
                /* Skip cert validation in insecure-test mode; resume handshake. */
                continue;
            }
            if (st == errSSLWouldBlock) {
                continue;
            }
            CFRelease(ctx);
            (void)close(fd);
            if (st == errSSLXCertChainInvalid || st == errSSLHostNameMismatch
                || st == errSSLPeerHandshakeFail || st == errSSLNegotiation) {
                return DRACONIC_HOST_E_CONN;
            }
            return DRACONIC_HOST_E_IO;
        }
    }

    err = host_handle_alloc(DRACONIC_HOST_HANDLE_KIND_TLS_CONN, fd, &tls_h);
    if (err != DRACONIC_HOST_OK) {
        (void)SSLClose(ctx);
        CFRelease(ctx);
        (void)close(fd);
        return err;
    }
    i = (size_t)tls_h - 1;
    g_host_handle_tls_ctx[i] = ctx;
    *out_tls = tls_h;
    return DRACONIC_HOST_OK;
}

/* H11.02: load PEM cert + key into a temporary keychain → SecIdentityRef. */
static CFDataRef host_tls_read_file_cfdata(const char *path) {
    FILE *f;
    long sz;
    uint8_t *buf;
    CFDataRef data;
    if (!path || path[0] == '\0') {
        return NULL;
    }
    f = fopen(path, "rb");
    if (!f) {
        return NULL;
    }
    if (fseek(f, 0, SEEK_END) != 0) {
        fclose(f);
        return NULL;
    }
    sz = ftell(f);
    if (sz < 0 || sz > 8 * 1024 * 1024) {
        fclose(f);
        return NULL;
    }
    if (fseek(f, 0, SEEK_SET) != 0) {
        fclose(f);
        return NULL;
    }
    buf = (uint8_t *)malloc((size_t)sz);
    if (!buf) {
        fclose(f);
        return NULL;
    }
    if (sz > 0 && fread(buf, 1, (size_t)sz, f) != (size_t)sz) {
        free(buf);
        fclose(f);
        return NULL;
    }
    fclose(f);
    data = CFDataCreate(NULL, buf, (CFIndex)sz);
    free(buf);
    return data;
}

static void host_tls_delete_keychain_file(const char *path) {
    if (path && path[0] != '\0') {
        (void)unlink(path);
        /* macOS may also leave -db companions; best-effort. */
        {
            char db[512];
            snprintf(db, sizeof(db), "%s-db", path);
            (void)unlink(db);
        }
    }
}

static DraconicHostError host_tls_identity_from_pem(
    const char *cert_path,
    const char *key_path,
    SecIdentityRef *out_identity,
    SecKeychainRef *out_kc,
    char *kc_path_buf,
    size_t kc_path_buf_len) {
    CFDataRef cert_data = NULL;
    CFDataRef key_data = NULL;
    SecKeychainRef kc = NULL;
    CFArrayRef cert_items = NULL;
    CFArrayRef key_items = NULL;
    SecCertificateRef cert = NULL;
    SecIdentityRef identity = NULL;
    SecExternalFormat fmt;
    SecExternalItemType itype;
    SecItemImportExportKeyParameters params;
    OSStatus st;
    CFIndex i, n;
    pid_t pid = getpid();

    if (!out_identity || !out_kc || !kc_path_buf || kc_path_buf_len < 64) {
        return DRACONIC_HOST_E_INVAL;
    }
    *out_identity = NULL;
    *out_kc = NULL;
    kc_path_buf[0] = '\0';

    cert_data = host_tls_read_file_cfdata(cert_path);
    key_data = host_tls_read_file_cfdata(key_path);
    if (!cert_data || !key_data) {
        if (cert_data) {
            CFRelease(cert_data);
        }
        if (key_data) {
            CFRelease(key_data);
        }
        return DRACONIC_HOST_E_IO;
    }

    snprintf(
        kc_path_buf,
        kc_path_buf_len,
        "/tmp/draconic-tls-%d-%ld.keychain",
        (int)pid,
        (long)time(NULL));
    host_tls_delete_keychain_file(kc_path_buf);
    st = SecKeychainCreate(kc_path_buf, 4, "test", false, NULL, &kc);
    if (st != errSecSuccess || !kc) {
        CFRelease(cert_data);
        CFRelease(key_data);
        kc_path_buf[0] = '\0';
        return DRACONIC_HOST_E_IO;
    }
    (void)SecKeychainUnlock(kc, 4, "test", true);

    memset(&params, 0, sizeof(params));
    params.version = SEC_KEY_IMPORT_EXPORT_PARAMS_VERSION;
    fmt = kSecFormatPEMSequence;
    itype = kSecItemTypeAggregate;
    st = SecItemImport(cert_data, CFSTR("cert.pem"), &fmt, &itype, 0, &params, kc, &cert_items);
    CFRelease(cert_data);
    if (st != errSecSuccess || !cert_items) {
        CFRelease(key_data);
        SecKeychainDelete(kc);
        CFRelease(kc);
        host_tls_delete_keychain_file(kc_path_buf);
        kc_path_buf[0] = '\0';
        return DRACONIC_HOST_E_IO;
    }

    /* PKCS#8 ("BEGIN PRIVATE KEY") often needs BSAFE; traditional RSA PEM uses PEMSequence. */
    {
        SecExternalFormat key_fmts[4];
        int fi;
        key_fmts[0] = kSecFormatBSAFE;
        key_fmts[1] = kSecFormatPEMSequence;
        key_fmts[2] = kSecFormatOpenSSL;
        key_fmts[3] = kSecFormatUnknown;
        st = errSecUnknownFormat;
        for (fi = 0; fi < 4; fi++) {
            fmt = key_fmts[fi];
            itype = kSecItemTypeAggregate;
            if (key_items) {
                CFRelease(key_items);
                key_items = NULL;
            }
            st = SecItemImport(
                key_data, CFSTR("key.pem"), &fmt, &itype, 0, &params, kc, &key_items);
            if (st == errSecSuccess && key_items && CFArrayGetCount(key_items) > 0) {
                break;
            }
            st = errSecUnknownFormat;
        }
    }
    CFRelease(key_data);
    if (key_items) {
        CFRelease(key_items);
        key_items = NULL;
    }
    if (st != errSecSuccess) {
        CFRelease(cert_items);
        SecKeychainDelete(kc);
        CFRelease(kc);
        host_tls_delete_keychain_file(kc_path_buf);
        kc_path_buf[0] = '\0';
        return DRACONIC_HOST_E_IO;
    }

    n = CFArrayGetCount(cert_items);
    for (i = 0; i < n; i++) {
        CFTypeRef item = CFArrayGetValueAtIndex(cert_items, i);
        if (item && CFGetTypeID(item) == SecCertificateGetTypeID()) {
            cert = (SecCertificateRef)item;
            CFRetain(cert);
            break;
        }
    }
    CFRelease(cert_items);
    if (!cert) {
        SecKeychainDelete(kc);
        CFRelease(kc);
        host_tls_delete_keychain_file(kc_path_buf);
        kc_path_buf[0] = '\0';
        return DRACONIC_HOST_E_IO;
    }

    st = SecIdentityCreateWithCertificate(kc, cert, &identity);
    CFRelease(cert);
    if (st != errSecSuccess || !identity) {
        SecKeychainDelete(kc);
        CFRelease(kc);
        host_tls_delete_keychain_file(kc_path_buf);
        kc_path_buf[0] = '\0';
        return DRACONIC_HOST_E_IO;
    }

    *out_identity = identity;
    *out_kc = kc;
    return DRACONIC_HOST_OK;
}

DraconicHostError draconic_rt_host_tls_server_wrap(
    DraconicHostHandle tcp_conn,
    const char *cert_path,
    const char *key_path,
    DraconicHostHandle *out_tls) {
    int fd = -1;
    SSLContextRef ctx = NULL;
    OSStatus st;
    DraconicHostError err;
    DraconicHostHandle tls_h = DRACONIC_HOST_HANDLE_INVALID;
    size_t i;
    SecIdentityRef identity = NULL;
    SecKeychainRef kc = NULL;
    char kc_path[512];
    CFArrayRef certs = NULL;

    if (!out_tls) {
        return DRACONIC_HOST_E_INVAL;
    }
    *out_tls = DRACONIC_HOST_HANDLE_INVALID;
    if (!cert_path || !key_path) {
        return DRACONIC_HOST_E_INVAL;
    }

    err = host_tls_identity_from_pem(cert_path, key_path, &identity, &kc, kc_path, sizeof(kc_path));
    if (err != DRACONIC_HOST_OK) {
        return err;
    }

    err = host_tls_steal_tcp_fd(tcp_conn, &fd);
    if (err != DRACONIC_HOST_OK) {
        CFRelease(identity);
        SecKeychainDelete(kc);
        CFRelease(kc);
        host_tls_delete_keychain_file(kc_path);
        return err;
    }

    ctx = SSLCreateContext(NULL, kSSLServerSide, kSSLStreamType);
    if (!ctx) {
        (void)close(fd);
        CFRelease(identity);
        SecKeychainDelete(kc);
        CFRelease(kc);
        host_tls_delete_keychain_file(kc_path);
        return DRACONIC_HOST_E_NOMEM;
    }
    st = SSLSetIOFuncs(ctx, host_tls_io_read, host_tls_io_write);
    if (st != noErr) {
        CFRelease(ctx);
        (void)close(fd);
        CFRelease(identity);
        SecKeychainDelete(kc);
        CFRelease(kc);
        host_tls_delete_keychain_file(kc_path);
        return DRACONIC_HOST_E_IO;
    }
    st = SSLSetConnection(ctx, (SSLConnectionRef)(intptr_t)fd);
    if (st != noErr) {
        CFRelease(ctx);
        (void)close(fd);
        CFRelease(identity);
        SecKeychainDelete(kc);
        CFRelease(kc);
        host_tls_delete_keychain_file(kc_path);
        return DRACONIC_HOST_E_IO;
    }

    certs = CFArrayCreate(NULL, (const void **)&identity, 1, &kCFTypeArrayCallBacks);
    CFRelease(identity);
    identity = NULL;
    if (!certs) {
        CFRelease(ctx);
        (void)close(fd);
        SecKeychainDelete(kc);
        CFRelease(kc);
        host_tls_delete_keychain_file(kc_path);
        return DRACONIC_HOST_E_NOMEM;
    }
    st = SSLSetCertificate(ctx, certs);
    CFRelease(certs);
    if (st != noErr) {
        CFRelease(ctx);
        (void)close(fd);
        SecKeychainDelete(kc);
        CFRelease(kc);
        host_tls_delete_keychain_file(kc_path);
        return DRACONIC_HOST_E_IO;
    }

    /* Bound handshake so plain-TCP peers fail closed instead of hanging. */
    {
        struct timeval tv;
        tv.tv_sec = 2;
        tv.tv_usec = 0;
        (void)setsockopt(fd, SOL_SOCKET, SO_RCVTIMEO, &tv, sizeof(tv));
        (void)setsockopt(fd, SOL_SOCKET, SO_SNDTIMEO, &tv, sizeof(tv));
    }

    {
        time_t deadline = time(NULL) + 3;
        for (;;) {
            if (time(NULL) > deadline) {
                CFRelease(ctx);
                (void)close(fd);
                SecKeychainDelete(kc);
                CFRelease(kc);
                host_tls_delete_keychain_file(kc_path);
                return DRACONIC_HOST_E_IO;
            }
            st = SSLHandshake(ctx);
            if (st == noErr) {
                break;
            }
            if (st == errSSLWouldBlock) {
                continue;
            }
            CFRelease(ctx);
            (void)close(fd);
            SecKeychainDelete(kc);
            CFRelease(kc);
            host_tls_delete_keychain_file(kc_path);
            if (st == errSSLPeerHandshakeFail || st == errSSLNegotiation
                || st == errSSLClosedAbort || st == errSSLClosedGraceful) {
                return DRACONIC_HOST_E_CONN;
            }
            return DRACONIC_HOST_E_IO;
        }
    }

    /* SSL context retains certificate material; drop temp keychain after handshake. */
    SecKeychainDelete(kc);
    CFRelease(kc);
    host_tls_delete_keychain_file(kc_path);
    kc = NULL;

    err = host_handle_alloc(DRACONIC_HOST_HANDLE_KIND_TLS_CONN, fd, &tls_h);
    if (err != DRACONIC_HOST_OK) {
        (void)SSLClose(ctx);
        CFRelease(ctx);
        (void)close(fd);
        return err;
    }
    i = (size_t)tls_h - 1;
    g_host_handle_tls_ctx[i] = ctx;
    *out_tls = tls_h;
    return DRACONIC_HOST_OK;
}

DraconicHostError draconic_rt_host_tls_read(
    DraconicHostHandle tls_h,
    size_t max_len,
    uint8_t **out_data,
    size_t *out_len) {
    SSLContextRef ctx;
    uint8_t *buf = NULL;
    size_t processed = 0;
    OSStatus st;

    if (!out_data || !out_len) {
        return DRACONIC_HOST_E_INVAL;
    }
    *out_data = NULL;
    *out_len = 0;
    if (max_len == 0) {
        return DRACONIC_HOST_OK;
    }
    ctx = host_handle_tls_ctx(tls_h);
    if (!ctx) {
        return DRACONIC_HOST_E_BADF;
    }
    buf = (uint8_t *)malloc(max_len);
    if (!buf) {
        return DRACONIC_HOST_E_NOMEM;
    }
    st = SSLRead(ctx, buf, max_len, &processed);
    if (st == errSSLClosedGraceful || st == errSSLClosedNoNotify) {
        free(buf);
        *out_data = NULL;
        *out_len = 0;
        return DRACONIC_HOST_OK;
    }
    if (st != noErr && st != errSSLWouldBlock) {
        free(buf);
        return DRACONIC_HOST_E_IO;
    }
    if (processed == 0) {
        free(buf);
        *out_data = NULL;
        *out_len = 0;
        return DRACONIC_HOST_OK;
    }
    *out_data = buf;
    *out_len = processed;
    return DRACONIC_HOST_OK;
}

DraconicHostError draconic_rt_host_tls_write(
    DraconicHostHandle tls_h,
    const uint8_t *data,
    size_t len) {
    SSLContextRef ctx;
    size_t off = 0;
    OSStatus st;

    ctx = host_handle_tls_ctx(tls_h);
    if (!ctx) {
        return DRACONIC_HOST_E_BADF;
    }
    if (len == 0) {
        return DRACONIC_HOST_OK;
    }
    if (!data) {
        return DRACONIC_HOST_E_INVAL;
    }
    while (off < len) {
        size_t processed = 0;
        st = SSLWrite(ctx, data + off, len - off, &processed);
        if (st != noErr && st != errSSLWouldBlock) {
            return DRACONIC_HOST_E_IO;
        }
        if (processed == 0 && st == errSSLWouldBlock) {
            continue;
        }
        if (processed == 0) {
            return DRACONIC_HOST_E_IO;
        }
        off += processed;
    }
    return DRACONIC_HOST_OK;
}

#else /* !__APPLE__ */

static void host_tls_ctx_dispose(void *ctx) {
    (void)ctx;
}

DraconicHostError draconic_rt_host_tls_client_wrap(
    DraconicHostHandle tcp_conn,
    const char *server_name,
    int32_t insecure,
    DraconicHostHandle *out_tls) {
    (void)tcp_conn;
    (void)server_name;
    (void)insecure;
    if (out_tls) {
        *out_tls = DRACONIC_HOST_HANDLE_INVALID;
    }
    return DRACONIC_HOST_E_NOSYS;
}

DraconicHostError draconic_rt_host_tls_server_wrap(
    DraconicHostHandle tcp_conn,
    const char *cert_path,
    const char *key_path,
    DraconicHostHandle *out_tls) {
    (void)tcp_conn;
    (void)cert_path;
    (void)key_path;
    if (out_tls) {
        *out_tls = DRACONIC_HOST_HANDLE_INVALID;
    }
    return DRACONIC_HOST_E_NOSYS;
}

DraconicHostError draconic_rt_host_tls_read(
    DraconicHostHandle tls_h,
    size_t max_len,
    uint8_t **out_data,
    size_t *out_len) {
    (void)tls_h;
    (void)max_len;
    if (out_data) {
        *out_data = NULL;
    }
    if (out_len) {
        *out_len = 0;
    }
    return DRACONIC_HOST_E_NOSYS;
}

DraconicHostError draconic_rt_host_tls_write(
    DraconicHostHandle tls_h,
    const uint8_t *data,
    size_t len) {
    (void)tls_h;
    (void)data;
    (void)len;
    return DRACONIC_HOST_E_NOSYS;
}

#endif /* __APPLE__ */
