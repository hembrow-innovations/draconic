/* Draconic Native Runtime C ABI (N05: GC + minimal std). */
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

#ifdef __cplusplus
}
#endif

#endif /* DRACONIC_RT_H */
