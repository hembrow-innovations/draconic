#include "draconic_rt.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <stdint.h>

/* Native Runtime C ABI (N05: GC + minimal std; N06.01: job queue). Linked into LLVM native binaries. */

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

/* --- GC hello (B09): tracing heap for JS strings and objects --- */

typedef enum {
    DRACONIC_TAG_STRING = 1,
    DRACONIC_TAG_OBJECT = 2,
} DraconicTag;

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

static void free_value(DraconicValue *v) {
    if (v->tag == DRACONIC_TAG_STRING && v->as.string.data) {
        free(v->as.string.data);
        v->as.string.data = NULL;
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
    /* B09: strings/objects have no child pointers yet */
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
