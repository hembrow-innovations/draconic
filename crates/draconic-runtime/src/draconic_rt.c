#include "draconic_rt.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>

/* Native Runtime C ABI (N05–N06.10). Linked into LLVM native binaries. */

void draconic_rt_hello(void) {
    puts("hello");
}

/* N01: print native integers for conformance observation (decimal + newline). */
void draconic_rt_print_i64(int64_t v) {
    printf("%lld\n", (long long)v);
}

void draconic_rt_print_u64(uint64_t v) {
    printf("%llu\n", (unsigned long long)v);
}

/* N02: print native floats / bool for conformance observation. */
void draconic_rt_print_f64(double v) {
    printf("%.17g\n", v);
}

void draconic_rt_print_bool(int8_t v) {
    puts(v ? "true" : "false");
}

void draconic_rt_print_str(const char *s) {
    if (!s) {
        puts("");
        return;
    }
    puts(s);
}

/* N08.02.08: C-string helpers for for-in/of over strings + concat observations. */
size_t draconic_rt_cstr_len(const char *s) {
    return s ? strlen(s) : 0;
}

char *draconic_rt_cstr_concat(const char *a, const char *b) {
    size_t la = a ? strlen(a) : 0;
    size_t lb = b ? strlen(b) : 0;
    char *out = (char *)malloc(la + lb + 1);
    if (!out) {
        abort();
    }
    if (la) {
        memcpy(out, a, la);
    }
    if (lb) {
        memcpy(out + la, b, lb);
    }
    out[la + lb] = '\0';
    return out;
}

char *draconic_rt_cstr_from_u64(uint64_t n) {
    char buf[32];
    int len = snprintf(buf, sizeof(buf), "%llu", (unsigned long long)n);
    if (len < 0) {
        abort();
    }
    char *out = (char *)malloc((size_t)len + 1);
    if (!out) {
        abort();
    }
    memcpy(out, buf, (size_t)len + 1);
    return out;
}

char *draconic_rt_cstr_from_code_unit(const char *s, size_t index) {
    char *out = (char *)malloc(2);
    if (!out) {
        abort();
    }
    out[0] = (s != NULL) ? s[index] : '\0';
    out[1] = '\0';
    return out;
}

/* --- WTF-8 / UTF-16 helpers (N08.07.05) --- */

static size_t jsstr_decode_units(const char *s, size_t byte_len, uint16_t *out, size_t out_cap) {
    size_t n = 0;
    size_t i = 0;
    if (!s) {
        return 0;
    }
    while (i < byte_len) {
        unsigned char c = (unsigned char)s[i];
        uint32_t cp;
        size_t adv;
        if (c < 0x80u) {
            cp = c;
            adv = 1;
        } else if ((c & 0xE0u) == 0xC0u && i + 1 < byte_len) {
            cp = ((uint32_t)(c & 0x1Fu) << 6) | (uint32_t)((unsigned char)s[i + 1] & 0x3Fu);
            adv = 2;
        } else if ((c & 0xF0u) == 0xE0u && i + 2 < byte_len) {
            cp = ((uint32_t)(c & 0x0Fu) << 12) | ((uint32_t)((unsigned char)s[i + 1] & 0x3Fu) << 6) |
                 (uint32_t)((unsigned char)s[i + 2] & 0x3Fu);
            adv = 3;
        } else if ((c & 0xF8u) == 0xF0u && i + 3 < byte_len) {
            cp = ((uint32_t)(c & 0x07u) << 18) | ((uint32_t)((unsigned char)s[i + 1] & 0x3Fu) << 12) |
                 ((uint32_t)((unsigned char)s[i + 2] & 0x3Fu) << 6) |
                 (uint32_t)((unsigned char)s[i + 3] & 0x3Fu);
            adv = 4;
        } else {
            cp = 0xFFFDu;
            adv = 1;
        }
        i += adv;
        if (cp <= 0xFFFFu) {
            if (out && n < out_cap) {
                out[n] = (uint16_t)cp;
            }
            n += 1;
        } else if (cp <= 0x10FFFFu) {
            uint32_t c2 = cp - 0x10000u;
            if (out && n + 1 < out_cap) {
                out[n] = (uint16_t)(0xD800u + (c2 >> 10));
                out[n + 1] = (uint16_t)(0xDC00u + (c2 & 0x3FFu));
            }
            n += 2;
        } else {
            if (out && n < out_cap) {
                out[n] = 0xFFFDu;
            }
            n += 1;
        }
    }
    return n;
}

static size_t jsstr_encode_unit(uint16_t u, unsigned char *out) {
    if (u < 0x80u) {
        out[0] = (unsigned char)u;
        return 1;
    }
    if (u < 0x800u) {
        out[0] = (unsigned char)(0xC0u | (u >> 6));
        out[1] = (unsigned char)(0x80u | (u & 0x3Fu));
        return 2;
    }
    /* BMP incl. unpaired surrogates (WTF-8). */
    out[0] = (unsigned char)(0xE0u | (u >> 12));
    out[1] = (unsigned char)(0x80u | ((u >> 6) & 0x3Fu));
    out[2] = (unsigned char)(0x80u | (u & 0x3Fu));
    return 3;
}

static size_t jsstr_encode_units(const uint16_t *units, size_t n, unsigned char *out) {
    size_t o = 0;
    size_t i = 0;
    while (i < n) {
        uint16_t u = units[i];
        if (u >= 0xD800u && u <= 0xDBFFu && i + 1 < n) {
            uint16_t v = units[i + 1];
            if (v >= 0xDC00u && v <= 0xDFFFu) {
                uint32_t cp = 0x10000u + (((uint32_t)(u - 0xD800u) << 10) | (uint32_t)(v - 0xDC00u));
                out[o++] = (unsigned char)(0xF0u | (cp >> 18));
                out[o++] = (unsigned char)(0x80u | ((cp >> 12) & 0x3Fu));
                out[o++] = (unsigned char)(0x80u | ((cp >> 6) & 0x3Fu));
                out[o++] = (unsigned char)(0x80u | (cp & 0x3Fu));
                i += 2;
                continue;
            }
        }
        o += jsstr_encode_unit(u, out + o);
        i += 1;
    }
    return o;
}

static size_t jsstr_encode_units_len(const uint16_t *units, size_t n) {
    size_t o = 0;
    size_t i = 0;
    while (i < n) {
        uint16_t u = units[i];
        if (u >= 0xD800u && u <= 0xDBFFu && i + 1 < n) {
            uint16_t v = units[i + 1];
            if (v >= 0xDC00u && v <= 0xDFFFu) {
                o += 4;
                i += 2;
                continue;
            }
        }
        if (u < 0x80u) {
            o += 1;
        } else if (u < 0x800u) {
            o += 2;
        } else {
            o += 3;
        }
        i += 1;
    }
    return o;
}

void draconic_rt_print_bytes(const char *s, size_t len) {
    /* Lossy UTF-8: unpaired surrogates → U+FFFD (matches Node stdout). */
    size_t nu = jsstr_decode_units(s, len, NULL, 0);
    uint16_t *units = NULL;
    if (nu) {
        units = (uint16_t *)malloc(nu * sizeof(uint16_t));
        if (!units) {
            abort();
        }
        jsstr_decode_units(s, len, units, nu);
    }
    size_t i = 0;
    while (i < nu) {
        uint16_t u = units[i];
        if (u >= 0xD800u && u <= 0xDBFFu && i + 1 < nu) {
            uint16_t v = units[i + 1];
            if (v >= 0xDC00u && v <= 0xDFFFu) {
                uint32_t cp = 0x10000u + (((uint32_t)(u - 0xD800u) << 10) | (uint32_t)(v - 0xDC00u));
                unsigned char buf[4];
                buf[0] = (unsigned char)(0xF0u | (cp >> 18));
                buf[1] = (unsigned char)(0x80u | ((cp >> 12) & 0x3Fu));
                buf[2] = (unsigned char)(0x80u | ((cp >> 6) & 0x3Fu));
                buf[3] = (unsigned char)(0x80u | (cp & 0x3Fu));
                fwrite(buf, 1, 4, stdout);
                i += 2;
                continue;
            }
        }
        if (u >= 0xD800u && u <= 0xDFFFu) {
            /* U+FFFD */
            fputs("\xEF\xBF\xBD", stdout);
        } else {
            unsigned char buf[3];
            size_t n = jsstr_encode_unit(u, buf);
            fwrite(buf, 1, n, stdout);
        }
        i += 1;
    }
    free(units);
    fputc('\n', stdout);
}

char *draconic_rt_cstr_concat_n(const char *a, size_t la, const char *b, size_t lb, size_t *out_len) {
    size_t na = jsstr_decode_units(a, la, NULL, 0);
    size_t nb = jsstr_decode_units(b, lb, NULL, 0);
    size_t n = na + nb;
    uint16_t *units = (uint16_t *)malloc((n ? n : 1) * sizeof(uint16_t));
    if (!units) {
        abort();
    }
    if (na) {
        jsstr_decode_units(a, la, units, na);
    }
    if (nb) {
        jsstr_decode_units(b, lb, units + na, nb);
    }
    size_t blen = jsstr_encode_units_len(units, n);
    char *out = (char *)malloc(blen + 1);
    if (!out) {
        abort();
    }
    jsstr_encode_units(units, n, (unsigned char *)out);
    out[blen] = '\0';
    free(units);
    if (out_len) {
        *out_len = blen;
    }
    return out;
}

char *draconic_rt_cstr_from_code_unit_n(const char *s, size_t len, size_t index, size_t *out_len) {
    size_t nu = jsstr_decode_units(s, len, NULL, 0);
    if (index >= nu) {
        char *out = (char *)malloc(1);
        if (!out) {
            abort();
        }
        out[0] = '\0';
        if (out_len) {
            *out_len = 0;
        }
        return out;
    }
    uint16_t *units = (uint16_t *)malloc(nu * sizeof(uint16_t));
    if (!units) {
        abort();
    }
    jsstr_decode_units(s, len, units, nu);
    unsigned char buf[3];
    size_t blen = jsstr_encode_unit(units[index], buf);
    free(units);
    char *out = (char *)malloc(blen + 1);
    if (!out) {
        abort();
    }
    memcpy(out, buf, blen);
    out[blen] = '\0';
    if (out_len) {
        *out_len = blen;
    }
    return out;
}

int draconic_rt_cstr_eq_n(const char *a, size_t la, const char *b, size_t lb) {
    size_t na = jsstr_decode_units(a, la, NULL, 0);
    size_t nb = jsstr_decode_units(b, lb, NULL, 0);
    if (na != nb) {
        return 0;
    }
    if (na == 0) {
        return 1;
    }
    uint16_t *ua = (uint16_t *)malloc(na * sizeof(uint16_t));
    uint16_t *ub = (uint16_t *)malloc(nb * sizeof(uint16_t));
    if (!ua || !ub) {
        abort();
    }
    jsstr_decode_units(a, la, ua, na);
    jsstr_decode_units(b, lb, ub, nb);
    int eq = memcmp(ua, ub, na * sizeof(uint16_t)) == 0;
    free(ua);
    free(ub);
    return eq;
}

size_t draconic_rt_utf16_len(const char *s, size_t byte_len) {
    return jsstr_decode_units(s, byte_len, NULL, 0);
}

/* --- GC hello (B09): tracing heap for JS strings and objects --- */

typedef enum {
    DRACONIC_TAG_STRING = 1,
    DRACONIC_TAG_OBJECT = 2,
    DRACONIC_TAG_PROMISE = 3,
    DRACONIC_TAG_ARRAY = 4,
} DraconicTag;

typedef struct DraconicPromiseReaction DraconicPromiseReaction;
typedef struct DraconicProp DraconicProp;

struct DraconicPromiseReaction {
    DraconicPromiseReactionFn on_fulfilled;
    void *fulfill_data;
    DraconicPromiseReactionFn on_rejected;
    void *reject_data;
    struct DraconicValue *derived;
    int finally_mode; /* N06.05: both paths run one callback; pass through settle */
    DraconicPromiseReaction *next;
};

/* N06.08: property list — string keys (heap C strings) or symbol keys (i64 id).
 * String props: key != NULL, symbol_id unused. Symbol props: key == NULL. */
struct DraconicProp {
    char *key;
    int64_t symbol_id;
    void *value;
    DraconicProp *next;
};

struct DraconicValue {
    DraconicTag tag;
    int marked;
    struct DraconicValue *next; /* intrusive list of all heap objects */
    union {
        struct {
            size_t len;
            char *data; /* heap-owned UTF-8 bytes, not null-required */
        } string;
        struct {
            DraconicProp *props; /* N06.08 own properties */
            struct DraconicValue *proto; /* N08.04.05 [[Prototype]]; nullable */
        } object;
        struct {
            int state; /* DRACONIC_PROMISE_* */
            void *result;
            DraconicPromiseReaction *reactions_head;
            DraconicPromiseReaction *reactions_tail;
        } promise;
        struct {
            size_t len;
            void **elems; /* heap-owned; opaque values (GC ptr or inttoptr) */
        } array;
    } as;
};

typedef struct DraconicValue DraconicValue;

static DraconicValue *g_heap_head = NULL;
static size_t g_live_count = 0;
static int g_gc_inited = 0;

#define ROOT_STACK_MAX 64
static DraconicValue *g_roots[ROOT_STACK_MAX];
static size_t g_root_sp = 0;

void draconic_rt_gc_init(void) {
    g_heap_head = NULL;
    g_live_count = 0;
    g_root_sp = 0;
    g_gc_inited = 1;
}

static void free_promise_reactions(DraconicPromiseReaction *head) {
    while (head) {
        DraconicPromiseReaction *next = head->next;
        free(head);
        head = next;
    }
}

static void free_props(DraconicProp *head) {
    while (head) {
        DraconicProp *next = head->next;
        if (head->key) {
            free(head->key);
        }
        free(head);
        head = next;
    }
}

static void free_value(DraconicValue *v) {
    if (v->tag == DRACONIC_TAG_STRING && v->as.string.data) {
        free(v->as.string.data);
        v->as.string.data = NULL;
    } else if (v->tag == DRACONIC_TAG_OBJECT) {
        free_props(v->as.object.props);
        v->as.object.props = NULL;
        v->as.object.proto = NULL;
    } else if (v->tag == DRACONIC_TAG_PROMISE) {
        free_promise_reactions(v->as.promise.reactions_head);
        v->as.promise.reactions_head = NULL;
        v->as.promise.reactions_tail = NULL;
    } else if (v->tag == DRACONIC_TAG_ARRAY) {
        free(v->as.array.elems);
        v->as.array.elems = NULL;
        v->as.array.len = 0;
    }
    free(v);
}

void draconic_rt_gc_shutdown(void) {
    DraconicValue *cur = g_heap_head;
    while (cur) {
        DraconicValue *next = cur->next;
        free_value(cur);
        cur = next;
    }
    g_heap_head = NULL;
    g_live_count = 0;
    g_root_sp = 0;
    g_gc_inited = 0;
}

static DraconicValue *heap_alloc(DraconicTag tag) {
    if (!g_gc_inited) {
        draconic_rt_gc_init();
    }
    DraconicValue *v = (DraconicValue *)calloc(1, sizeof(DraconicValue));
    if (!v) {
        return NULL;
    }
    v->tag = tag;
    v->marked = 0;
    v->next = g_heap_head;
    g_heap_head = v;
    g_live_count++;
    return v;
}

DraconicValue *draconic_rt_alloc_string(const char *data, size_t len) {
    DraconicValue *v = heap_alloc(DRACONIC_TAG_STRING);
    if (!v) {
        return NULL;
    }
    char *buf = (char *)malloc(len ? len : 1);
    if (!buf) {
        /* roll back header */
        g_heap_head = v->next;
        g_live_count--;
        free(v);
        return NULL;
    }
    if (len && data) {
        memcpy(buf, data, len);
    }
    v->as.string.len = len;
    v->as.string.data = buf;
    return v;
}

DraconicValue *draconic_rt_alloc_object(void) {
    return heap_alloc(DRACONIC_TAG_OBJECT);
}

void draconic_rt_gc_root_push(DraconicValue *v) {
    if (g_root_sp >= ROOT_STACK_MAX) {
        fprintf(stderr, "draconic_rt: root stack overflow\n");
        abort();
    }
    g_roots[g_root_sp++] = v;
}

void draconic_rt_gc_root_pop(void) {
    if (g_root_sp == 0) {
        fprintf(stderr, "draconic_rt: root stack underflow\n");
        abort();
    }
    g_root_sp--;
    g_roots[g_root_sp] = NULL;
}

static int is_heap_value(DraconicValue *v) {
    if (!v) {
        return 0;
    }
    for (DraconicValue *cur = g_heap_head; cur; cur = cur->next) {
        if (cur == v) {
            return 1;
        }
    }
    return 0;
}

static void mark_value(DraconicValue *v) {
    if (!v || !is_heap_value(v) || v->marked) {
        return;
    }
    v->marked = 1;
    if (v->tag == DRACONIC_TAG_OBJECT) {
        /* N08.04.05 / N09.02: keep [[Prototype]] and own prop values live. */
        mark_value(v->as.object.proto);
        for (DraconicProp *p = v->as.object.props; p; p = p->next) {
            mark_value((DraconicValue *)p->value);
        }
    } else if (v->tag == DRACONIC_TAG_PROMISE) {
        DraconicPromiseReaction *r = v->as.promise.reactions_head;
        while (r) {
            mark_value(r->derived);
            r = r->next;
        }
        mark_value((DraconicValue *)v->as.promise.result);
    } else if (v->tag == DRACONIC_TAG_ARRAY && v->as.array.elems) {
        for (size_t i = 0; i < v->as.array.len; i++) {
            mark_value((DraconicValue *)v->as.array.elems[i]);
        }
    }
}

void draconic_rt_gc_collect(void) {
    /* mark */
    for (size_t i = 0; i < g_root_sp; i++) {
        mark_value(g_roots[i]);
    }

    /* sweep */
    DraconicValue **link = &g_heap_head;
    while (*link) {
        DraconicValue *cur = *link;
        if (!cur->marked) {
            *link = cur->next;
            free_value(cur);
            g_live_count--;
        } else {
            cur->marked = 0;
            link = &cur->next;
        }
    }
}

size_t draconic_rt_gc_live_count(void) {
    return g_live_count;
}

const char *draconic_rt_string_data(DraconicValue *v) {
    if (!v || v->tag != DRACONIC_TAG_STRING) {
        return NULL;
    }
    return v->as.string.data;
}

size_t draconic_rt_string_len(DraconicValue *v) {
    if (!v || v->tag != DRACONIC_TAG_STRING) {
        return 0;
    }
    return v->as.string.len;
}

int draconic_rt_is_string(DraconicValue *v) {
    return is_heap_value(v) && v->tag == DRACONIC_TAG_STRING;
}

int draconic_rt_is_object(DraconicValue *v) {
    return is_heap_value(v) && v->tag == DRACONIC_TAG_OBJECT;
}

/* --- Job queue (N06.01): FIFO host jobs; drain until empty --- */

typedef struct DraconicJob {
    DraconicJobFn fn;
    void *data;
    struct DraconicJob *next;
} DraconicJob;

static DraconicJob *g_job_head = NULL;
static DraconicJob *g_job_tail = NULL;
static size_t g_job_pending = 0;
static int g_job_draining = 0;

void draconic_rt_job_enqueue(DraconicJobFn fn, void *data) {
    if (!fn) {
        fprintf(stderr, "draconic_rt: job_enqueue null fn\n");
        abort();
    }
    DraconicJob *job = (DraconicJob *)calloc(1, sizeof(DraconicJob));
    if (!job) {
        fprintf(stderr, "draconic_rt: job_enqueue OOM\n");
        abort();
    }
    job->fn = fn;
    job->data = data;
    job->next = NULL;
    if (g_job_tail) {
        g_job_tail->next = job;
    } else {
        g_job_head = job;
    }
    g_job_tail = job;
    g_job_pending++;
}

size_t draconic_rt_job_pending(void) {
    return g_job_pending;
}

void draconic_rt_job_drain(void) {
    if (g_job_draining) {
        /* Re-entrant drain is a no-op; nested enqueues stay on the queue
           for the outer drain to continue processing. */
        return;
    }
    g_job_draining = 1;
    while (g_job_head) {
        DraconicJob *job = g_job_head;
        g_job_head = job->next;
        if (!g_job_head) {
            g_job_tail = NULL;
        }
        g_job_pending--;
        DraconicJobFn fn = job->fn;
        void *data = job->data;
        free(job);
        fn(data);
    }
    g_job_draining = 0;
}

/* --- Promise (N06.02): pending → settle once; then reactions as jobs --- */

void draconic_rt_promise_resolve(DraconicValue *p, void *value);
void draconic_rt_promise_reject(DraconicValue *p, void *reason);

typedef struct {
    DraconicPromiseReactionFn fn;
    void *data;
    void *arg;
    int reject_passthrough; /* no handler on reject → reject derived with arg */
    int finally_pass; /* call fn for side effects; settle derived with original arg */
    DraconicValue *derived;
} PromiseReactionJob;

static void promise_reaction_job(void *data) {
    PromiseReactionJob *job = (PromiseReactionJob *)data;
    DraconicValue *derived = job->derived;
    if (!derived || derived->tag != DRACONIC_TAG_PROMISE) {
        free(job);
        return;
    }
    if (job->finally_pass) {
        if (job->fn) {
            (void)job->fn(job->data, job->arg);
        }
        if (job->reject_passthrough) {
            draconic_rt_promise_reject(derived, job->arg);
        } else {
            draconic_rt_promise_resolve(derived, job->arg);
        }
    } else if (job->fn) {
        void *out = job->fn(job->data, job->arg);
        draconic_rt_promise_resolve(derived, out);
    } else if (job->reject_passthrough) {
        draconic_rt_promise_reject(derived, job->arg);
    } else {
        draconic_rt_promise_resolve(derived, job->arg);
    }
    free(job);
}

static void enqueue_promise_reaction(
    DraconicPromiseReactionFn fn,
    void *data,
    void *arg,
    int reject_passthrough,
    DraconicValue *derived
) {
    PromiseReactionJob *job = (PromiseReactionJob *)calloc(1, sizeof(PromiseReactionJob));
    if (!job) {
        fprintf(stderr, "draconic_rt: promise reaction OOM\n");
        abort();
    }
    job->fn = fn;
    job->data = data;
    job->arg = arg;
    job->reject_passthrough = reject_passthrough;
    job->finally_pass = 0;
    job->derived = derived;
    draconic_rt_job_enqueue(promise_reaction_job, job);
}

static void enqueue_promise_finally_reaction(
    DraconicPromiseReactionFn fn,
    void *data,
    void *arg,
    int is_reject,
    DraconicValue *derived
) {
    PromiseReactionJob *job = (PromiseReactionJob *)calloc(1, sizeof(PromiseReactionJob));
    if (!job) {
        fprintf(stderr, "draconic_rt: promise finally OOM\n");
        abort();
    }
    job->fn = fn;
    job->data = data;
    job->arg = arg;
    job->reject_passthrough = is_reject ? 1 : 0;
    job->finally_pass = 1;
    job->derived = derived;
    draconic_rt_job_enqueue(promise_reaction_job, job);
}

static void promise_settle(DraconicValue *p, int state, void *result) {
    if (!p || p->tag != DRACONIC_TAG_PROMISE) {
        return;
    }
    if (p->as.promise.state != DRACONIC_PROMISE_PENDING) {
        return;
    }
    p->as.promise.state = state;
    p->as.promise.result = result;

    DraconicPromiseReaction *r = p->as.promise.reactions_head;
    p->as.promise.reactions_head = NULL;
    p->as.promise.reactions_tail = NULL;

    while (r) {
        DraconicPromiseReaction *next = r->next;
        if (r->finally_mode) {
            enqueue_promise_finally_reaction(
                r->on_fulfilled,
                r->fulfill_data,
                result,
                state == DRACONIC_PROMISE_REJECTED ? 1 : 0,
                r->derived
            );
        } else if (state == DRACONIC_PROMISE_FULFILLED) {
            enqueue_promise_reaction(
                r->on_fulfilled,
                r->fulfill_data,
                result,
                0,
                r->derived
            );
        } else {
            enqueue_promise_reaction(
                r->on_rejected,
                r->reject_data,
                result,
                r->on_rejected == NULL ? 1 : 0,
                r->derived
            );
        }
        free(r);
        r = next;
    }
}

DraconicValue *draconic_rt_promise_new(void) {
    DraconicValue *v = heap_alloc(DRACONIC_TAG_PROMISE);
    if (!v) {
        return NULL;
    }
    v->as.promise.state = DRACONIC_PROMISE_PENDING;
    v->as.promise.result = NULL;
    v->as.promise.reactions_head = NULL;
    v->as.promise.reactions_tail = NULL;
    return v;
}

int draconic_rt_is_promise(DraconicValue *v) {
    return is_heap_value(v) && v->tag == DRACONIC_TAG_PROMISE;
}

int draconic_rt_promise_state(DraconicValue *p) {
    if (!p || p->tag != DRACONIC_TAG_PROMISE) {
        return -1;
    }
    return p->as.promise.state;
}

void *draconic_rt_promise_result(DraconicValue *p) {
    if (!p || p->tag != DRACONIC_TAG_PROMISE) {
        return NULL;
    }
    return p->as.promise.result;
}

void draconic_rt_promise_resolve(DraconicValue *p, void *value) {
    promise_settle(p, DRACONIC_PROMISE_FULFILLED, value);
}

void draconic_rt_promise_reject(DraconicValue *p, void *reason) {
    promise_settle(p, DRACONIC_PROMISE_REJECTED, reason);
}

DraconicValue *draconic_rt_promise_then(
    DraconicValue *p,
    DraconicPromiseReactionFn on_fulfilled,
    void *fulfill_data,
    DraconicPromiseReactionFn on_rejected,
    void *reject_data
) {
    DraconicValue *derived = draconic_rt_promise_new();
    if (!derived) {
        return NULL;
    }
    if (!p || p->tag != DRACONIC_TAG_PROMISE) {
        draconic_rt_promise_reject(derived, NULL);
        return derived;
    }

    int state = p->as.promise.state;
    if (state == DRACONIC_PROMISE_PENDING) {
        DraconicPromiseReaction *r =
            (DraconicPromiseReaction *)calloc(1, sizeof(DraconicPromiseReaction));
        if (!r) {
            fprintf(stderr, "draconic_rt: promise_then OOM\n");
            abort();
        }
        r->on_fulfilled = on_fulfilled;
        r->fulfill_data = fulfill_data;
        r->on_rejected = on_rejected;
        r->reject_data = reject_data;
        r->derived = derived;
        r->finally_mode = 0;
        r->next = NULL;
        if (p->as.promise.reactions_tail) {
            p->as.promise.reactions_tail->next = r;
        } else {
            p->as.promise.reactions_head = r;
        }
        p->as.promise.reactions_tail = r;
        return derived;
    }

    if (state == DRACONIC_PROMISE_FULFILLED) {
        enqueue_promise_reaction(
            on_fulfilled,
            fulfill_data,
            p->as.promise.result,
            0,
            derived
        );
    } else {
        enqueue_promise_reaction(
            on_rejected,
            reject_data,
            p->as.promise.result,
            on_rejected == NULL ? 1 : 0,
            derived
        );
    }
    return derived;
}

/* --- Promise construct with executor (N06.03) --- */

static void promise_settle_resolve_cap(void *capability, void *value) {
    draconic_rt_promise_resolve((DraconicValue *)capability, value);
}

static void promise_settle_reject_cap(void *capability, void *reason) {
    draconic_rt_promise_reject((DraconicValue *)capability, reason);
}

DraconicValue *draconic_rt_promise_construct(
    DraconicPromiseExecutorFn executor,
    void *data
) {
    DraconicValue *p = draconic_rt_promise_new();
    if (!p) {
        return NULL;
    }
    if (!executor) {
        return p;
    }
    executor(
        data,
        promise_settle_resolve_cap,
        (void *)p,
        promise_settle_reject_cap,
        (void *)p
    );
    return p;
}

/* --- Promise.prototype.finally (N06.05) --- */

DraconicValue *draconic_rt_promise_finally(
    DraconicValue *p,
    DraconicPromiseReactionFn on_finally,
    void *data
) {
    DraconicValue *derived = draconic_rt_promise_new();
    if (!derived) {
        return NULL;
    }
    if (!p || p->tag != DRACONIC_TAG_PROMISE) {
        draconic_rt_promise_reject(derived, NULL);
        return derived;
    }

    int state = p->as.promise.state;
    if (state == DRACONIC_PROMISE_PENDING) {
        DraconicPromiseReaction *r =
            (DraconicPromiseReaction *)calloc(1, sizeof(DraconicPromiseReaction));
        if (!r) {
            fprintf(stderr, "draconic_rt: promise_finally OOM\n");
            abort();
        }
        r->on_fulfilled = on_finally;
        r->fulfill_data = data;
        r->on_rejected = on_finally;
        r->reject_data = data;
        r->derived = derived;
        r->finally_mode = 1;
        r->next = NULL;
        if (p->as.promise.reactions_tail) {
            p->as.promise.reactions_tail->next = r;
        } else {
            p->as.promise.reactions_head = r;
        }
        p->as.promise.reactions_tail = r;
        return derived;
    }

    enqueue_promise_finally_reaction(
        on_finally,
        data,
        p->as.promise.result,
        state == DRACONIC_PROMISE_REJECTED ? 1 : 0,
        derived
    );
    return derived;
}

/* --- JS arrays (N06.06) --- */

DraconicValue *draconic_rt_array_new(size_t len) {
    DraconicValue *v = heap_alloc(DRACONIC_TAG_ARRAY);
    if (!v) {
        return NULL;
    }
    v->as.array.len = len;
    v->as.array.elems = NULL;
    if (len > 0) {
        v->as.array.elems = (void **)calloc(len, sizeof(void *));
        if (!v->as.array.elems) {
            g_heap_head = v->next;
            g_live_count--;
            free(v);
            return NULL;
        }
    }
    return v;
}

int draconic_rt_is_array(DraconicValue *v) {
    return is_heap_value(v) && v->tag == DRACONIC_TAG_ARRAY;
}

size_t draconic_rt_array_len(DraconicValue *a) {
    if (!a || a->tag != DRACONIC_TAG_ARRAY) {
        return 0;
    }
    return a->as.array.len;
}

void *draconic_rt_array_get(DraconicValue *a, size_t index) {
    if (!a || a->tag != DRACONIC_TAG_ARRAY || index >= a->as.array.len) {
        return NULL;
    }
    return a->as.array.elems[index];
}

void draconic_rt_array_set(DraconicValue *a, size_t index, void *value) {
    if (!a || a->tag != DRACONIC_TAG_ARRAY) {
        return;
    }
    /* JS: assign beyond length grows the array (holes are NULL). */
    if (index >= a->as.array.len) {
        size_t new_len = index + 1;
        void **elems = (void **)realloc(a->as.array.elems, new_len * sizeof(void *));
        if (!elems) {
            fprintf(stderr, "draconic_rt: array_set OOM\n");
            abort();
        }
        for (size_t i = a->as.array.len; i < new_len; i++) {
            elems[i] = NULL;
        }
        a->as.array.elems = elems;
        a->as.array.len = new_len;
    }
    a->as.array.elems[index] = value;
}

/* N08.06.03: `[...arr]` — append all elements of src onto dest. */
void draconic_rt_array_spread_array(DraconicValue *dest, DraconicValue *src) {
    if (!dest || dest->tag != DRACONIC_TAG_ARRAY || !src || src->tag != DRACONIC_TAG_ARRAY) {
        return;
    }
    size_t n = src->as.array.len;
    size_t base = dest->as.array.len;
    for (size_t i = 0; i < n; i++) {
        void *el = src->as.array.elems ? src->as.array.elems[i] : NULL;
        draconic_rt_array_set(dest, base + i, el);
    }
}

/* N08.06.03: `[...str]` — append each code unit as a 1-char C string. */
void draconic_rt_array_spread_cstr(DraconicValue *dest, const char *s) {
    if (!dest || dest->tag != DRACONIC_TAG_ARRAY) {
        return;
    }
    size_t n = s ? strlen(s) : 0;
    size_t base = dest->as.array.len;
    for (size_t i = 0; i < n; i++) {
        char *ch = draconic_rt_cstr_from_code_unit(s, i);
        draconic_rt_array_set(dest, base + i, ch);
    }
}

/* --- Promise.all (N06.06) --- */

typedef struct {
    DraconicValue *all_promise;
    DraconicValue *results;
    size_t *remaining;
    int *rejected_flag;
    size_t index;
} PromiseAllSlot;

static void *promise_all_on_fulfill(void *data, void *value) {
    PromiseAllSlot *slot = (PromiseAllSlot *)data;
    if (!slot || !slot->remaining || !slot->rejected_flag) {
        return value;
    }
    if (*slot->rejected_flag) {
        return value;
    }
    draconic_rt_array_set(slot->results, slot->index, value);
    if (*slot->remaining > 0) {
        (*slot->remaining)--;
    }
    if (*slot->remaining == 0) {
        draconic_rt_promise_resolve(slot->all_promise, slot->results);
    }
    return value;
}

static void *promise_all_on_reject(void *data, void *reason) {
    PromiseAllSlot *slot = (PromiseAllSlot *)data;
    if (!slot || !slot->rejected_flag) {
        return reason;
    }
    if (!*slot->rejected_flag) {
        *slot->rejected_flag = 1;
        draconic_rt_promise_reject(slot->all_promise, reason);
    }
    return reason;
}

DraconicValue *draconic_rt_promise_all(DraconicValue *arr) {
    DraconicValue *out = draconic_rt_promise_new();
    if (!out) {
        return NULL;
    }
    size_t n = 0;
    if (arr && arr->tag == DRACONIC_TAG_ARRAY) {
        n = arr->as.array.len;
    }
    if (n == 0) {
        DraconicValue *empty = draconic_rt_array_new(0);
        draconic_rt_promise_resolve(out, empty);
        return out;
    }

    DraconicValue *results = draconic_rt_array_new(n);
    if (!results) {
        draconic_rt_promise_reject(out, NULL);
        return out;
    }

    size_t *remaining = (size_t *)malloc(sizeof(size_t));
    int *rejected_flag = (int *)malloc(sizeof(int));
    if (!remaining || !rejected_flag) {
        free(remaining);
        free(rejected_flag);
        draconic_rt_promise_reject(out, NULL);
        return out;
    }
    *remaining = n;
    *rejected_flag = 0;

    for (size_t i = 0; i < n; i++) {
        void *elem = draconic_rt_array_get(arr, i);
        PromiseAllSlot *slot = (PromiseAllSlot *)calloc(1, sizeof(PromiseAllSlot));
        if (!slot) {
            fprintf(stderr, "draconic_rt: promise_all OOM\n");
            abort();
        }
        slot->all_promise = out;
        slot->results = results;
        slot->remaining = remaining;
        slot->rejected_flag = rejected_flag;
        slot->index = i;

        if (draconic_rt_is_promise((DraconicValue *)elem)) {
            (void)draconic_rt_promise_then(
                (DraconicValue *)elem,
                promise_all_on_fulfill,
                slot,
                promise_all_on_reject,
                slot
            );
        } else {
            /* Non-thenable: Promise.resolve(elem) then wait (async via job). */
            DraconicValue *wrapped = draconic_rt_promise_new();
            draconic_rt_promise_resolve(wrapped, elem);
            (void)draconic_rt_promise_then(
                wrapped,
                promise_all_on_fulfill,
                slot,
                promise_all_on_reject,
                slot
            );
        }
    }
    return out;
}

/* --- Promise.race (N06.07) --- */

typedef struct {
    DraconicValue *race_promise;
    int *settled_flag;
} PromiseRaceSlot;

static void *promise_race_on_fulfill(void *data, void *value) {
    PromiseRaceSlot *slot = (PromiseRaceSlot *)data;
    if (!slot || !slot->settled_flag) {
        return value;
    }
    if (!*slot->settled_flag) {
        *slot->settled_flag = 1;
        draconic_rt_promise_resolve(slot->race_promise, value);
    }
    return value;
}

static void *promise_race_on_reject(void *data, void *reason) {
    PromiseRaceSlot *slot = (PromiseRaceSlot *)data;
    if (!slot || !slot->settled_flag) {
        return reason;
    }
    if (!*slot->settled_flag) {
        *slot->settled_flag = 1;
        draconic_rt_promise_reject(slot->race_promise, reason);
    }
    return reason;
}

DraconicValue *draconic_rt_promise_race(DraconicValue *arr) {
    DraconicValue *out = draconic_rt_promise_new();
    if (!out) {
        return NULL;
    }
    size_t n = 0;
    if (arr && arr->tag == DRACONIC_TAG_ARRAY) {
        n = arr->as.array.len;
    }
    /* Empty iterable: result stays pending (ECMA-262). */
    if (n == 0) {
        return out;
    }

    int *settled_flag = (int *)malloc(sizeof(int));
    if (!settled_flag) {
        draconic_rt_promise_reject(out, NULL);
        return out;
    }
    *settled_flag = 0;

    for (size_t i = 0; i < n; i++) {
        void *elem = draconic_rt_array_get(arr, i);
        PromiseRaceSlot *slot = (PromiseRaceSlot *)calloc(1, sizeof(PromiseRaceSlot));
        if (!slot) {
            fprintf(stderr, "draconic_rt: promise_race OOM\n");
            abort();
        }
        slot->race_promise = out;
        slot->settled_flag = settled_flag;

        if (draconic_rt_is_promise((DraconicValue *)elem)) {
            (void)draconic_rt_promise_then(
                (DraconicValue *)elem,
                promise_race_on_fulfill,
                slot,
                promise_race_on_reject,
                slot
            );
        } else {
            DraconicValue *wrapped = draconic_rt_promise_new();
            draconic_rt_promise_resolve(wrapped, elem);
            (void)draconic_rt_promise_then(
                wrapped,
                promise_race_on_fulfill,
                slot,
                promise_race_on_reject,
                slot
            );
        }
    }
    return out;
}

/* --- Object properties (N06.08) --- */

void draconic_rt_object_set(DraconicValue *obj, const char *key, void *value) {
    if (!obj || obj->tag != DRACONIC_TAG_OBJECT || !key) {
        return;
    }
    for (DraconicProp *p = obj->as.object.props; p; p = p->next) {
        if (p->key && strcmp(p->key, key) == 0) {
            p->value = value;
            return;
        }
    }
    DraconicProp *prop = (DraconicProp *)calloc(1, sizeof(DraconicProp));
    if (!prop) {
        fprintf(stderr, "draconic_rt: object_set OOM\n");
        abort();
    }
    size_t klen = strlen(key);
    prop->key = (char *)malloc(klen + 1);
    if (!prop->key) {
        free(prop);
        fprintf(stderr, "draconic_rt: object_set OOM\n");
        abort();
    }
    memcpy(prop->key, key, klen + 1);
    prop->symbol_id = 0;
    prop->value = value;
    prop->next = obj->as.object.props;
    obj->as.object.props = prop;
}

void *draconic_rt_object_get(DraconicValue *obj, const char *key) {
    if (!obj || !key) {
        return NULL;
    }
    /* N08.16.25: array exotic [[Get]] for decimal indexes + "length" (inttoptr). */
    if (obj->tag == DRACONIC_TAG_ARRAY) {
        if (strcmp(key, "length") == 0) {
            return (void *)(intptr_t)obj->as.array.len;
        }
        char *end = NULL;
        unsigned long idx = strtoul(key, &end, 10);
        if (end && end != key && *end == '\0') {
            return draconic_rt_array_get(obj, (size_t)idx);
        }
        return NULL;
    }
    if (obj->tag != DRACONIC_TAG_OBJECT) {
        return NULL;
    }
    /* N08.04.05: ordinary [[Get]] walks [[Prototype]] for missing own keys. */
    for (DraconicValue *cur = obj; cur && cur->tag == DRACONIC_TAG_OBJECT; cur = cur->as.object.proto) {
        for (DraconicProp *p = cur->as.object.props; p; p = p->next) {
            if (p->key && strcmp(p->key, key) == 0) {
                return p->value;
            }
        }
    }
    return NULL;
}

DraconicValue *draconic_rt_object_rest(DraconicValue *obj, const char **exclude) {
    DraconicValue *out = draconic_rt_alloc_object();
    if (!out || !obj || obj->tag != DRACONIC_TAG_OBJECT) {
        return out;
    }
    for (DraconicProp *p = obj->as.object.props; p; p = p->next) {
        if (!p->key) {
            continue;
        }
        int skip = 0;
        if (exclude) {
            for (const char **e = exclude; *e; e++) {
                if (strcmp(p->key, *e) == 0) {
                    skip = 1;
                    break;
                }
            }
        }
        if (!skip) {
            draconic_rt_object_set(out, p->key, p->value);
        }
    }
    return out;
}

void draconic_rt_object_set_symbol(DraconicValue *obj, int64_t sym, void *value) {
    if (!obj || obj->tag != DRACONIC_TAG_OBJECT || sym == 0) {
        return;
    }
    for (DraconicProp *p = obj->as.object.props; p; p = p->next) {
        if (!p->key && p->symbol_id == sym) {
            p->value = value;
            return;
        }
    }
    DraconicProp *prop = (DraconicProp *)calloc(1, sizeof(DraconicProp));
    if (!prop) {
        fprintf(stderr, "draconic_rt: object_set_symbol OOM\n");
        abort();
    }
    prop->key = NULL;
    prop->symbol_id = sym;
    prop->value = value;
    prop->next = obj->as.object.props;
    obj->as.object.props = prop;
}

void *draconic_rt_object_get_symbol(DraconicValue *obj, int64_t sym) {
    if (!obj || obj->tag != DRACONIC_TAG_OBJECT || sym == 0) {
        return NULL;
    }
    for (DraconicValue *cur = obj; cur && cur->tag == DRACONIC_TAG_OBJECT; cur = cur->as.object.proto) {
        for (DraconicProp *p = cur->as.object.props; p; p = p->next) {
            if (!p->key && p->symbol_id == sym) {
                return p->value;
            }
        }
    }
    return NULL;
}

void draconic_rt_object_set_proto(DraconicValue *obj, DraconicValue *proto) {
    if (!obj || obj->tag != DRACONIC_TAG_OBJECT) {
        return;
    }
    if (proto && proto->tag != DRACONIC_TAG_OBJECT) {
        return;
    }
    obj->as.object.proto = proto;
}

DraconicValue *draconic_rt_object_get_proto(DraconicValue *obj) {
    if (!obj || obj->tag != DRACONIC_TAG_OBJECT) {
        return NULL;
    }
    return obj->as.object.proto;
}

/* N08.16.19: copy own string-keyed props (shallow) for object rest. */
void draconic_rt_object_copy_own(DraconicValue *dst, DraconicValue *src) {
    if (!dst || !src || dst->tag != DRACONIC_TAG_OBJECT || src->tag != DRACONIC_TAG_OBJECT) {
        return;
    }
    /* Copy in reverse so insertion order matches src after prepend-set. */
    size_t n = 0;
    for (DraconicProp *p = src->as.object.props; p; p = p->next) {
        if (p->key) {
            n++;
        }
    }
    if (n == 0) {
        return;
    }
    const char **keys = (const char **)calloc(n, sizeof(const char *));
    void **vals = (void **)calloc(n, sizeof(void *));
    if (!keys || !vals) {
        free(keys);
        free(vals);
        fprintf(stderr, "draconic_rt: object_copy_own OOM\n");
        abort();
    }
    size_t i = n;
    for (DraconicProp *p = src->as.object.props; p; p = p->next) {
        if (p->key) {
            i--;
            keys[i] = p->key;
            vals[i] = p->value;
        }
    }
    for (i = 0; i < n; i++) {
        draconic_rt_object_set(dst, keys[i], vals[i]);
    }
    free(keys);
    free(vals);
}

void draconic_rt_object_spread(DraconicValue *dest, DraconicValue *src) {
    if (!dest || dest->tag != DRACONIC_TAG_OBJECT) {
        return;
    }
    if (!src || src->tag != DRACONIC_TAG_OBJECT) {
        return;
    }
    size_t n = 0;
    for (DraconicProp *p = src->as.object.props; p; p = p->next) {
        if (p->key) {
            n++;
        }
    }
    if (n == 0) {
        return;
    }
    DraconicProp **ordered = (DraconicProp **)malloc(n * sizeof(DraconicProp *));
    if (!ordered) {
        fprintf(stderr, "draconic_rt: object_spread OOM\n");
        abort();
    }
    size_t i = n;
    for (DraconicProp *p = src->as.object.props; p; p = p->next) {
        if (p->key) {
            ordered[--i] = p;
        }
    }
    for (size_t j = 0; j < n; j++) {
        draconic_rt_object_set(dest, ordered[j]->key, ordered[j]->value);
    }
    free(ordered);
}

void draconic_rt_object_delete(DraconicValue *obj, const char *key) {
    if (!obj || obj->tag != DRACONIC_TAG_OBJECT || !key) {
        return;
    }
    DraconicProp **pp = &obj->as.object.props;
    while (*pp) {
        DraconicProp *p = *pp;
        if (p->key && strcmp(p->key, key) == 0) {
            *pp = p->next;
            free(p->key);
            free(p);
            return;
        }
        pp = &p->next;
    }
}

/* --- Promise.allSettled (N06.08) --- */

typedef struct {
    DraconicValue *all_promise;
    DraconicValue *results;
    size_t *remaining;
    size_t index;
} PromiseAllSettledSlot;

static DraconicValue *make_settled_result(const char *status, const char *payload_key, void *payload) {
    DraconicValue *obj = draconic_rt_alloc_object();
    if (!obj) {
        return NULL;
    }
    /* status is a static C string so print_str on string locals works. */
    draconic_rt_object_set(obj, "status", (void *)status);
    draconic_rt_object_set(obj, payload_key, payload);
    return obj;
}

static void *promise_all_settled_on_fulfill(void *data, void *value) {
    PromiseAllSettledSlot *slot = (PromiseAllSettledSlot *)data;
    if (!slot || !slot->remaining) {
        return value;
    }
    DraconicValue *entry = make_settled_result("fulfilled", "value", value);
    draconic_rt_array_set(slot->results, slot->index, entry);
    if (*slot->remaining > 0) {
        (*slot->remaining)--;
    }
    if (*slot->remaining == 0) {
        draconic_rt_promise_resolve(slot->all_promise, slot->results);
    }
    return value;
}

static void *promise_all_settled_on_reject(void *data, void *reason) {
    PromiseAllSettledSlot *slot = (PromiseAllSettledSlot *)data;
    if (!slot || !slot->remaining) {
        return reason;
    }
    DraconicValue *entry = make_settled_result("rejected", "reason", reason);
    draconic_rt_array_set(slot->results, slot->index, entry);
    if (*slot->remaining > 0) {
        (*slot->remaining)--;
    }
    if (*slot->remaining == 0) {
        draconic_rt_promise_resolve(slot->all_promise, slot->results);
    }
    return reason;
}

DraconicValue *draconic_rt_promise_all_settled(DraconicValue *arr) {
    DraconicValue *out = draconic_rt_promise_new();
    if (!out) {
        return NULL;
    }
    size_t n = 0;
    if (arr && arr->tag == DRACONIC_TAG_ARRAY) {
        n = arr->as.array.len;
    }
    if (n == 0) {
        DraconicValue *empty = draconic_rt_array_new(0);
        draconic_rt_promise_resolve(out, empty);
        return out;
    }

    DraconicValue *results = draconic_rt_array_new(n);
    if (!results) {
        draconic_rt_promise_reject(out, NULL);
        return out;
    }

    size_t *remaining = (size_t *)malloc(sizeof(size_t));
    if (!remaining) {
        draconic_rt_promise_reject(out, NULL);
        return out;
    }
    *remaining = n;

    for (size_t i = 0; i < n; i++) {
        void *elem = draconic_rt_array_get(arr, i);
        PromiseAllSettledSlot *slot =
            (PromiseAllSettledSlot *)calloc(1, sizeof(PromiseAllSettledSlot));
        if (!slot) {
            fprintf(stderr, "draconic_rt: promise_all_settled OOM\n");
            abort();
        }
        slot->all_promise = out;
        slot->results = results;
        slot->remaining = remaining;
        slot->index = i;

        if (draconic_rt_is_promise((DraconicValue *)elem)) {
            (void)draconic_rt_promise_then(
                (DraconicValue *)elem,
                promise_all_settled_on_fulfill,
                slot,
                promise_all_settled_on_reject,
                slot
            );
        } else {
            DraconicValue *wrapped = draconic_rt_promise_new();
            draconic_rt_promise_resolve(wrapped, elem);
            (void)draconic_rt_promise_then(
                wrapped,
                promise_all_settled_on_fulfill,
                slot,
                promise_all_settled_on_reject,
                slot
            );
        }
    }
    return out;
}

/* --- Promise.any (N06.09) --- */

static DraconicValue *make_aggregate_error(DraconicValue *errors) {
    DraconicValue *err = draconic_rt_alloc_object();
    if (!err) {
        return NULL;
    }
    draconic_rt_object_set(err, "name", (void *)"AggregateError");
    draconic_rt_object_set(err, "errors", errors);
    return err;
}

typedef struct {
    DraconicValue *any_promise;
    DraconicValue *errors;
    size_t *remaining;
    int *fulfilled_flag;
    size_t index;
} PromiseAnySlot;

static void *promise_any_on_fulfill(void *data, void *value) {
    PromiseAnySlot *slot = (PromiseAnySlot *)data;
    if (!slot || !slot->fulfilled_flag) {
        return value;
    }
    if (!*slot->fulfilled_flag) {
        *slot->fulfilled_flag = 1;
        draconic_rt_promise_resolve(slot->any_promise, value);
    }
    return value;
}

static void *promise_any_on_reject(void *data, void *reason) {
    PromiseAnySlot *slot = (PromiseAnySlot *)data;
    if (!slot || !slot->remaining || !slot->fulfilled_flag) {
        return reason;
    }
    if (*slot->fulfilled_flag) {
        return reason;
    }
    draconic_rt_array_set(slot->errors, slot->index, reason);
    if (*slot->remaining > 0) {
        (*slot->remaining)--;
    }
    if (*slot->remaining == 0) {
        DraconicValue *agg = make_aggregate_error(slot->errors);
        draconic_rt_promise_reject(slot->any_promise, agg);
    }
    return reason;
}

DraconicValue *draconic_rt_promise_any(DraconicValue *arr) {
    DraconicValue *out = draconic_rt_promise_new();
    if (!out) {
        return NULL;
    }
    size_t n = 0;
    if (arr && arr->tag == DRACONIC_TAG_ARRAY) {
        n = arr->as.array.len;
    }
    if (n == 0) {
        DraconicValue *empty = draconic_rt_array_new(0);
        DraconicValue *agg = make_aggregate_error(empty);
        draconic_rt_promise_reject(out, agg);
        return out;
    }

    DraconicValue *errors = draconic_rt_array_new(n);
    if (!errors) {
        draconic_rt_promise_reject(out, NULL);
        return out;
    }

    size_t *remaining = (size_t *)malloc(sizeof(size_t));
    int *fulfilled_flag = (int *)malloc(sizeof(int));
    if (!remaining || !fulfilled_flag) {
        free(remaining);
        free(fulfilled_flag);
        draconic_rt_promise_reject(out, NULL);
        return out;
    }
    *remaining = n;
    *fulfilled_flag = 0;

    for (size_t i = 0; i < n; i++) {
        void *elem = draconic_rt_array_get(arr, i);
        PromiseAnySlot *slot = (PromiseAnySlot *)calloc(1, sizeof(PromiseAnySlot));
        if (!slot) {
            fprintf(stderr, "draconic_rt: promise_any OOM\n");
            abort();
        }
        slot->any_promise = out;
        slot->errors = errors;
        slot->remaining = remaining;
        slot->fulfilled_flag = fulfilled_flag;
        slot->index = i;

        if (draconic_rt_is_promise((DraconicValue *)elem)) {
            (void)draconic_rt_promise_then(
                (DraconicValue *)elem,
                promise_any_on_fulfill,
                slot,
                promise_any_on_reject,
                slot
            );
        } else {
            DraconicValue *wrapped = draconic_rt_promise_new();
            draconic_rt_promise_resolve(wrapped, elem);
            (void)draconic_rt_promise_then(
                wrapped,
                promise_any_on_fulfill,
                slot,
                promise_any_on_reject,
                slot
            );
        }
    }
    return out;
}

/* --- await operand (N06.10) --- */

DraconicValue *draconic_rt_promise_await(void *value) {
    if (draconic_rt_is_promise((DraconicValue *)value)) {
        return (DraconicValue *)value;
    }
    DraconicValue *p = draconic_rt_promise_new();
    if (!p) {
        return NULL;
    }
    draconic_rt_promise_resolve(p, value);
    return p;
}

/* --- JS Symbol (N08.09.01) --- */

typedef struct DraconicSymbolReg {
    char *key;
    size_t key_len;
    int64_t id;
    struct DraconicSymbolReg *next;
} DraconicSymbolReg;

static int64_t g_symbol_next = 1;
static DraconicSymbolReg *g_symbol_registry = NULL;

int64_t draconic_rt_symbol_new(void) {
    return g_symbol_next++;
}

int64_t draconic_rt_symbol_for(const char *key, size_t key_len) {
    const char *k = key ? key : "";
    for (DraconicSymbolReg *e = g_symbol_registry; e; e = e->next) {
        if (e->key_len == key_len && (key_len == 0 || memcmp(e->key, k, key_len) == 0)) {
            return e->id;
        }
    }
    DraconicSymbolReg *e = (DraconicSymbolReg *)calloc(1, sizeof(DraconicSymbolReg));
    if (!e) {
        abort();
    }
    e->key = (char *)malloc(key_len + 1);
    if (!e->key) {
        abort();
    }
    if (key_len) {
        memcpy(e->key, k, key_len);
    }
    e->key[key_len] = '\0';
    e->key_len = key_len;
    e->id = g_symbol_next++;
    e->next = g_symbol_registry;
    g_symbol_registry = e;
    return e->id;
}

char *draconic_rt_symbol_key_for(int64_t id, size_t *out_len) {
    for (DraconicSymbolReg *e = g_symbol_registry; e; e = e->next) {
        if (e->id == id) {
            char *out = (char *)malloc(e->key_len + 1);
            if (!out) {
                abort();
            }
            if (e->key_len) {
                memcpy(out, e->key, e->key_len);
            }
            out[e->key_len] = '\0';
            if (out_len) {
                *out_len = e->key_len;
            }
            return out;
        }
    }
    if (out_len) {
        *out_len = 0;
    }
    return NULL;
}
