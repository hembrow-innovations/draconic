/* Host I/O Runtime substrate ABI (H00.02).
   Error codes, opaque handles, UTF-8 path encoding at the OS boundary.
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

#ifdef __cplusplus
}
#endif

#endif /* DRACONIC_RT_HOST_H */
