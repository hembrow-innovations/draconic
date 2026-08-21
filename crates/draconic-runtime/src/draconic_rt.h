/* Draconic Native Runtime C ABI (N05–N06.09: GC, job queue, Promise + all/race/allSettled/any). */
#ifndef DRACONIC_RT_H
#define DRACONIC_RT_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* --- Minimal std I/O hooks --- */
void draconic_rt_hello(void);
/* F01.01: multi-arg i32 extern call target (C ABI; linked with runtime). */
int32_t draconic_rt_add_i32(int32_t a, int32_t b);
void draconic_rt_print_i64(int64_t v);
void draconic_rt_print_u64(uint64_t v);
void draconic_rt_print_f64(double v);
void draconic_rt_print_bool(int8_t v);
/* Print a NUL-terminated C string + newline (N06.03 observations). */
void draconic_rt_print_str(const char *s);

/* --- C-string helpers for ES expr native observations (N08.02.08 for-in/of) ---
   Results are malloc-owned (leaked OK for short tests).
   JS string storage is WTF-8 (UTF-8 + unpaired surrogates as 3-byte sequences).
   N08.07.05: index/concat/eq/print operate on UTF-16 code units. */
size_t draconic_rt_cstr_len(const char *s);
char *draconic_rt_cstr_concat(const char *a, const char *b);
char *draconic_rt_cstr_from_u64(uint64_t n);
char *draconic_rt_cstr_from_code_unit(const char *s, size_t index);
/* N08.07.01: length-aware bytes (embedded NUL / non-C-string JS strings). */
void draconic_rt_print_bytes(const char *s, size_t len);
/* Concat as UTF-16 units; *out_len is WTF-8 byte length of result. */
char *draconic_rt_cstr_concat_n(const char *a, size_t la, const char *b, size_t lb, size_t *out_len);
/* Index by UTF-16 code unit; *out_len is WTF-8 byte length (0 if OOB). */
char *draconic_rt_cstr_from_code_unit_n(const char *s, size_t len, size_t index, size_t *out_len);
/* Equality on UTF-16 code unit sequences. */
int draconic_rt_cstr_eq_n(const char *a, size_t la, const char *b, size_t lb);
/* N08.07.03/N08.07.05: JS `.length` = UTF-16 code units over WTF-8 storage. */
size_t draconic_rt_utf16_len(const char *s, size_t byte_len);

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
/* N09.05: auto-collect when live_count reaches threshold after alloc; 0 disables. */
void draconic_rt_gc_set_alloc_threshold(size_t threshold);
size_t draconic_rt_gc_alloc_threshold(void);

const char *draconic_rt_string_data(DraconicValue *v);
size_t draconic_rt_string_len(DraconicValue *v);
int draconic_rt_is_string(DraconicValue *v);
int draconic_rt_is_object(DraconicValue *v);

/* --- Job queue (Promise Jobs / microtasks; N06.01) --- */
typedef void (*DraconicJobFn)(void *data);
void draconic_rt_job_enqueue(DraconicJobFn fn, void *data);
void draconic_rt_job_drain(void);
size_t draconic_rt_job_pending(void);

/* --- OS sleep / yield (H16.04): for timer tests and job_drain waits ---
   sleep_ms blocks the thread for ~ms (capped per call); <= 0 or NaN is a no-op.
   yield voluntarily gives up the CPU slice without a timed wait. */
void draconic_rt_sleep_ms(double ms);
void draconic_rt_yield(void);

/* --- Host timers (H05.03–H05.05): setTimeout / setInterval via job queue ---
   Due timers are promoted into the job queue at the end of each drain
   wave (after microtasks). Delay is wall-clock ms; delay <= 0 is due
   immediately on the next promote. clearTimeout/clearInterval cancel by id
   (shared id space). Intervals reschedule after each run until cleared.
   job_drain waits (OS sleep) for future timers instead of busy-spinning
   or returning early while timers remain (H05.05).
   H07.01: job_drain also polls host IO readiness waits (see host.h
   io_wait/io_poll) and enqueues their completions as jobs. */
int64_t draconic_rt_timer_set(DraconicJobFn fn, void *data, double delay_ms);
int64_t draconic_rt_timer_set_interval(DraconicJobFn fn, void *data, double interval_ms);
void draconic_rt_timer_clear(int64_t id);

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

/* --- Promise construct with executor (N06.03 / `new Promise(executor)`) --- */
/* Settle callbacks passed into the executor (capability is the Promise*). */
typedef void (*DraconicPromiseSettleFn)(void *capability, void *value_or_reason);
/* Executor: may call resolve/reject synchronously (or schedule later). */
typedef void (*DraconicPromiseExecutorFn)(
    void *data,
    DraconicPromiseSettleFn resolve,
    void *resolve_cap,
    DraconicPromiseSettleFn reject,
    void *reject_cap);
/* Create a pending Promise and invoke `executor` with resolve/reject caps. */
DraconicValue *draconic_rt_promise_construct(
    DraconicPromiseExecutorFn executor,
    void *data);

/* --- Promise.prototype.finally (N06.05) --- */
/* Run `on_finally` on settle; pass through fulfillment value or rejection reason.
   Callback return is ignored (simple subset; thenables/throws deferred). */
DraconicValue *draconic_rt_promise_finally(
    DraconicValue *p,
    DraconicPromiseReactionFn on_finally,
    void *data);

/* --- JS arrays (N06.06; elements are opaque void* — numbers as inttoptr) --- */
DraconicValue *draconic_rt_array_new(size_t len);
int draconic_rt_is_array(DraconicValue *v);
size_t draconic_rt_array_len(DraconicValue *a);
void *draconic_rt_array_get(DraconicValue *a, size_t index);
void draconic_rt_array_set(DraconicValue *a, size_t index, void *value);
/* N08.06.03: append iterable elements onto dest (grows). */
void draconic_rt_array_spread_array(DraconicValue *dest, DraconicValue *src);
void draconic_rt_array_spread_cstr(DraconicValue *dest, const char *s);

/* --- Promise.all (N06.06): array of promises/values → promise of results array --- */
DraconicValue *draconic_rt_promise_all(DraconicValue *arr);

/* --- Promise.race (N06.07): array of promises/values → first settle wins --- */
DraconicValue *draconic_rt_promise_race(DraconicValue *arr);

/* --- Plain object props (N06.08; keys are NUL-terminated C strings) --- */
void draconic_rt_object_set(DraconicValue *obj, const char *key, void *value);
/* N08.04.05: [[Get]] walks [[Prototype]] when key is missing on own props. */
void *draconic_rt_object_get(DraconicValue *obj, const char *key);
/* N08.09.02: symbol-keyed own props (i64 Symbol id; no string collision). */
void draconic_rt_object_set_symbol(DraconicValue *obj, int64_t sym, void *value);
void *draconic_rt_object_get_symbol(DraconicValue *obj, int64_t sym);
/* N08.04.05: set/get ordinary object [[Prototype]] (nullable). */
void draconic_rt_object_set_proto(DraconicValue *obj, DraconicValue *proto);
DraconicValue *draconic_rt_object_get_proto(DraconicValue *obj);
/* N08.16.19: object rest — copy own string props then delete excluded keys. */
DraconicValue *draconic_rt_object_rest(DraconicValue *obj, const char **exclude);
void draconic_rt_object_copy_own(DraconicValue *dst, DraconicValue *src);
void draconic_rt_object_delete(DraconicValue *obj, const char *key);
void draconic_rt_object_spread(DraconicValue *dest, DraconicValue *src);

/* --- Promise.allSettled (N06.08): array of promises/values →
       promise of [{status,value|reason}, …] --- */
DraconicValue *draconic_rt_promise_all_settled(DraconicValue *arr);

/* --- Promise.any (N06.09): array of promises/values → first fulfillment;
       if all reject (or empty), reject AggregateError { name, errors } --- */
DraconicValue *draconic_rt_promise_any(DraconicValue *arr);

/* --- await operand (N06.10): if value is a Promise, return it; else wrap as
       fulfilled Promise (PromiseResolve of a non-thenable). --- */
DraconicValue *draconic_rt_promise_await(void *value);

/* --- JS Symbol (N08.09.01): unique i64 ids; Symbol.for registry --- */
/* Fresh unique symbol (description ignored for identity). */
int64_t draconic_rt_symbol_new(void);
/* Global registry: same key → same id. */
int64_t draconic_rt_symbol_for(const char *key, size_t key_len);
/* keyFor: malloc'd key bytes + *out_len; NULL if not from Symbol.for. */
char *draconic_rt_symbol_key_for(int64_t id, size_t *out_len);

/* Host I/O substrate (H00.02–H00.03): error codes, handles, path, bytes. */
#include "draconic_rt_host.h"

#ifdef __cplusplus
}
#endif

#endif /* DRACONIC_RT_H */
