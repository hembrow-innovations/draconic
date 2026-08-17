/* Host I/O Runtime substrate (H00.02–H00.03).
   Error codes, opaque handles, UTF-8 path encoding, I/O bytes boundary.
   No real tcp/fs/process — later H rows open handles and map errno. */

#include "draconic_rt_host.h"

#include <stdint.h>
#include <stdlib.h>
#include <string.h>

/* --- Handle table (slots filled by later open/listen/etc.) --- */

#define DRACONIC_HOST_HANDLE_SLOTS 256

/* Live flags for 1-based handle ids. H04/H06 open paths will set slots. */
static uint8_t g_host_handle_live[DRACONIC_HOST_HANDLE_SLOTS];

int draconic_rt_host_handle_is_valid(DraconicHostHandle h) {
    if (h < 1 || h > (DraconicHostHandle)DRACONIC_HOST_HANDLE_SLOTS) {
        return 0;
    }
    return g_host_handle_live[(size_t)h - 1] != 0;
}

DraconicHostError draconic_rt_host_handle_close(DraconicHostHandle h) {
    if (!draconic_rt_host_handle_is_valid(h)) {
        return DRACONIC_HOST_E_BADF;
    }
    g_host_handle_live[(size_t)h - 1] = 0;
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
