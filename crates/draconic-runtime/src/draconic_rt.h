/* Draconic Native Runtime C ABI (N05: GC + minimal std; N06.01: job queue; N06.02: Promise). */
#ifndef DRACONIC_RT_H
#define DRACONIC_RT_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* --- Minimal std I/O hooks --- */
void draconic_rt_hello(void);
void draconic_rt_print_i64(int64_t v);
void draconic_rt_print_u64(uint64_t v);
void draconic_rt_print_f64(double v);
void draconic_rt_print_bool(int8_t v);

/* --- GC heap for JS values --- */
typedef struct DraconicValue DraconicValue;

void draconic_rt_gc_init(void);
void draconic_rt_gc_shutdown(void);
DraconicValue *draconic_rt_alloc_string(const char *data, size_t len);
DraconicValue *draconic_rt_alloc_object(void);
void draconic_rt_gc_root_push(DraconicValue *v);
void draconic_rt_gc_root_pop(void);
void draconic_rt_gc_collect(void);
size_t draconic_rt_gc_live_count(void);

const char *draconic_rt_string_data(DraconicValue *v);
size_t draconic_rt_string_len(DraconicValue *v);
int draconic_rt_is_string(DraconicValue *v);
int draconic_rt_is_object(DraconicValue *v);

/* --- Job queue (Promise Jobs / microtasks; N06.01) --- */
typedef void (*DraconicJobFn)(void *data);
void draconic_rt_job_enqueue(DraconicJobFn fn, void *data);
void draconic_rt_job_drain(void);
size_t draconic_rt_job_pending(void);

/* --- Promise (N06.02): settle + then reactions via job queue --- */
#define DRACONIC_PROMISE_PENDING 0
#define DRACONIC_PROMISE_FULFILLED 1
#define DRACONIC_PROMISE_REJECTED 2

/* Reaction callback: return value fulfills the derived promise from `then`. */
typedef void *(*DraconicPromiseReactionFn)(void *data, void *value_or_reason);

DraconicValue *draconic_rt_promise_new(void);
int draconic_rt_is_promise(DraconicValue *v);
int draconic_rt_promise_state(DraconicValue *p);
void *draconic_rt_promise_result(DraconicValue *p);
void draconic_rt_promise_resolve(DraconicValue *p, void *value);
void draconic_rt_promise_reject(DraconicValue *p, void *reason);
/* Attach reactions; returns a new pending promise settled from the reaction. */
DraconicValue *draconic_rt_promise_then(
    DraconicValue *p,
    DraconicPromiseReactionFn on_fulfilled,
    void *fulfill_data,
    DraconicPromiseReactionFn on_rejected,
    void *reject_data);

#ifdef __cplusplus
}
#endif

#endif /* DRACONIC_RT_H */
