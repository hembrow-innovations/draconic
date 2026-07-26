#include "draconic_rt.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>

/* Native Runtime C ABI (N05–N06.03). Linked into LLVM native binaries. */

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

/* --- GC hello (B09): tracing heap for JS strings and objects --- */

typedef enum {
    DRACONIC_TAG_STRING = 1,
    DRACONIC_TAG_OBJECT = 2,
    DRACONIC_TAG_PROMISE = 3,
} DraconicTag;

typedef struct DraconicPromiseReaction DraconicPromiseReaction;

struct DraconicPromiseReaction {
    DraconicPromiseReactionFn on_fulfilled;
    void *fulfill_data;
    DraconicPromiseReactionFn on_rejected;
    void *reject_data;
    struct DraconicValue *derived;
    DraconicPromiseReaction *next;
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
            /* empty object for GC hello; properties come later */
            int _pad;
        } object;
        struct {
            int state; /* DRACONIC_PROMISE_* */
            void *result;
            DraconicPromiseReaction *reactions_head;
            DraconicPromiseReaction *reactions_tail;
        } promise;
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

static void free_value(DraconicValue *v) {
    if (v->tag == DRACONIC_TAG_STRING && v->as.string.data) {
        free(v->as.string.data);
        v->as.string.data = NULL;
    } else if (v->tag == DRACONIC_TAG_PROMISE) {
        free_promise_reactions(v->as.promise.reactions_head);
        v->as.promise.reactions_head = NULL;
        v->as.promise.reactions_tail = NULL;
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

static void mark_value(DraconicValue *v) {
    if (!v || v->marked) {
        return;
    }
    v->marked = 1;
    if (v->tag == DRACONIC_TAG_PROMISE) {
        DraconicPromiseReaction *r = v->as.promise.reactions_head;
        while (r) {
            mark_value(r->derived);
            r = r->next;
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
    return v && v->tag == DRACONIC_TAG_STRING;
}

int draconic_rt_is_object(DraconicValue *v) {
    return v && v->tag == DRACONIC_TAG_OBJECT;
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
    int reject_passthrough; /* fn == NULL on reject path → reject derived */
    DraconicValue *derived;
} PromiseReactionJob;

static void promise_reaction_job(void *data) {
    PromiseReactionJob *job = (PromiseReactionJob *)data;
    DraconicValue *derived = job->derived;
    if (!derived || derived->tag != DRACONIC_TAG_PROMISE) {
        free(job);
        return;
    }
    if (job->fn) {
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
        if (state == DRACONIC_PROMISE_FULFILLED) {
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
    return v && v->tag == DRACONIC_TAG_PROMISE;
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
