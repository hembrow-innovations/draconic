/* Host I/O Runtime substrate ABI (H00.02–H00.03).
   Error codes, opaque handles, UTF-8 path encoding, I/O bytes boundary.
   Included from draconic_rt.h; also usable standalone by host.c. */
#ifndef DRACONIC_RT_HOST_H
#define DRACONIC_RT_HOST_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef int32_t DraconicHostError;
typedef int64_t DraconicHostHandle;

#define DRACONIC_HOST_OK 0
#define DRACONIC_HOST_E_INVAL 1
#define DRACONIC_HOST_E_NOENT 2
#define DRACONIC_HOST_E_NOSYS 3
#define DRACONIC_HOST_E_BADF 4
#define DRACONIC_HOST_E_EXIST 5
#define DRACONIC_HOST_E_PERM 6
#define DRACONIC_HOST_E_IO 7
#define DRACONIC_HOST_E_NOMEM 8
#define DRACONIC_HOST_E_AGAIN 9
#define DRACONIC_HOST_E_CONN 10
#define DRACONIC_HOST_E_ADDR 11

#define DRACONIC_HOST_HANDLE_INVALID ((DraconicHostHandle)-1)

/* 1 if `h` refers to a live host resource; 0 otherwise. */
int draconic_rt_host_handle_is_valid(DraconicHostHandle h);
/* Close a live handle. Invalid/already-closed → DRACONIC_HOST_E_BADF. */
DraconicHostError draconic_rt_host_handle_close(DraconicHostHandle h);

/* Encode a JS/Draconic UTF-8 path at the OS boundary.
   On OK: *out_path is a malloc'd NUL-terminated byte string (caller frees
   with draconic_rt_host_path_free). Rejects embedded NUL and invalid UTF-8
   with DRACONIC_HOST_E_INVAL. Does not touch the filesystem. */
DraconicHostError draconic_rt_host_path_from_utf8(
    const char *utf8,
    size_t len,
    char **out_path);
void draconic_rt_host_path_free(char *path);

/* --- I/O bytes boundary (H00.03) -------------------------------------------
   Contiguous byte regions used as OS read/write buffers. Models ArrayBuffer
   storage and Uint8Array views (byteOffset + byteLength). Not C strings:
   embedded 0x00 is payload. Views borrow storage; they do not own it. */

typedef struct DraconicHostBytes {
    uint8_t *data;
    size_t len;
} DraconicHostBytes;

/* Borrow a view over raw storage. data may be NULL only when len == 0. */
DraconicHostError draconic_rt_host_bytes_from_raw(
    uint8_t *data,
    size_t len,
    DraconicHostBytes *out);

/* Uint8Array-style subview: parent[byte_offset, byte_offset + byte_length).
   Bounds-checked (offset+length must not exceed parent->len). */
DraconicHostError draconic_rt_host_bytes_view(
    const DraconicHostBytes *parent,
    size_t byte_offset,
    size_t byte_length,
    DraconicHostBytes *out);

/* Allocate zero-filled storage (ArrayBuffer-like backing).
   On OK: *out_data is malloc'd (len==0 → NULL). Free with
   draconic_rt_host_bytes_storage_free. */
DraconicHostError draconic_rt_host_bytes_alloc(
    size_t len,
    uint8_t **out_data);
void draconic_rt_host_bytes_storage_free(uint8_t *data);

/* OS read path: copy into the view from an external source.
   *out_n = min(dst->len, src_len) bytes written. */
DraconicHostError draconic_rt_host_bytes_copy_in(
    DraconicHostBytes *dst,
    const uint8_t *src,
    size_t src_len,
    size_t *out_n);

/* OS write path: copy out of the view into an external destination.
   *out_n = min(dst_cap, src->len) bytes written. */
DraconicHostError draconic_rt_host_bytes_copy_out(
    const DraconicHostBytes *src,
    uint8_t *dst,
    size_t dst_cap,
    size_t *out_n);

/* --- Process argv (H01.01) -------------------------------------------------
   Store OS argv at process start. User program args are argv[1..argc)
   (skip argv[0] program path). JS bridge mirrors Node user args. */

/* Record process argc/argv (pointers borrowed for process lifetime). */
void draconic_rt_host_process_set_argv(int argc, char **argv);
/* Number of user args (max(0, argc - 1)). */
int32_t draconic_rt_host_process_user_argc(void);
/* User arg at index i (0-based); NULL if out of range. */
const char *draconic_rt_host_process_user_arg(int32_t i);

/* --- Process env (H01.02) --------------------------------------------------
   String env get/set/delete. Missing get → NULL (JS undefined).
   env_get returns a freshly allocated copy (malloc); free with free() or
   ignore until process exit. */

/* malloc'd copy of env value, or NULL if missing / invalid key. */
char *draconic_rt_host_env_get(const char *key);
/* Set env key to value (overwrite). 0 ok, nonzero error. */
int32_t draconic_rt_host_env_set(const char *key, const char *value);
/* Delete env key. 0 ok (including already-missing), nonzero error. */
int32_t draconic_rt_host_env_delete(const char *key);

/* --- Process exit (H01.03) -------------------------------------------------
   Immediate terminate via exit(code). Deferred exitCode used when main returns
   without calling exit (default 0). */

/* Terminate process with status code (does not return). */
void draconic_rt_host_process_exit(int32_t code);
/* Set deferred exit status (returned from main if exit() not called). */
void draconic_rt_host_process_set_exit_code(int32_t code);
/* Get deferred exit status (default 0). */
int32_t draconic_rt_host_process_get_exit_code(void);

/* --- Process pid / ppid (H01.04) -------------------------------------------
   Read-only OS process id and parent process id. */

int32_t draconic_rt_host_process_pid(void);
int32_t draconic_rt_host_process_ppid(void);

#ifdef __cplusplus
}
#endif

#endif /* DRACONIC_RT_HOST_H */
