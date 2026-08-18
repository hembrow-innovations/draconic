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

/* --- Process signals (H14.01) ----------------------------------------------
   Portable signal codes (not necessarily OS signo values):
     DRACONIC_HOST_SIG_INT  = SIGINT
     DRACONIC_HOST_SIG_TERM = SIGTERM
   Default (no watch): OS default terminate (SIG_DFL) remains in effect.
   With watch: delivery sets a flag; job_drain promotes handlers onto the job
   queue (async-signal-safe path only flags; user code runs as a job). */

#define DRACONIC_HOST_SIG_INT 2
#define DRACONIC_HOST_SIG_TERM 15

typedef void (*DraconicHostSignalFn)(void *data);

/* Install watch for SIGINT or SIGTERM. Handler runs via job queue after poll.
   Replaces prior watch for that signal. fn must be non-NULL. */
DraconicHostError draconic_rt_host_signal_watch(
    int32_t sig,
    DraconicHostSignalFn fn,
    void *data);
/* Raise signal to the current process (tests / self-notify). */
DraconicHostError draconic_rt_host_signal_raise(int32_t sig);
/* Promote pending signal deliveries into job queue. Returns jobs enqueued.
   Called from draconic_rt_job_drain. */
int draconic_rt_host_signal_poll(void);

/* --- Wall clock (H05.01) ---------------------------------------------------
   Milliseconds since Unix epoch (UTC), as IEEE-754 double (JS Number). */

double draconic_rt_host_now_ms(void);

/* --- Monotonic clock (H05.02) ----------------------------------------------
   Milliseconds from an arbitrary epoch (steady while process runs). For
   durations only — not comparable to wall clock / Date.now. */

double draconic_rt_host_monotonic_ms(void);

/* --- Stdout write (H02.01) -------------------------------------------------
   Write raw bytes to OS stdout. data may be NULL only when len == 0.
   Binary-safe (embedded 0x00 is payload). No automatic newline. */

DraconicHostError draconic_rt_host_stdout_write(const uint8_t *data, size_t len);

/* --- Stderr write (H02.02) -------------------------------------------------
   Write raw bytes to OS stderr. data may be NULL only when len == 0.
   Binary-safe (embedded 0x00 is payload). No automatic newline. */

DraconicHostError draconic_rt_host_stderr_write(const uint8_t *data, size_t len);

/* --- Stdin read (H02.03) ---------------------------------------------------
   Blocking reads from OS stdin. v1: no timeouts. */

/* Read one line (through `\n` or EOF). On success: malloc'd bytes without
   trailing `\n` or `\r\n` (caller frees with free). Empty line → empty string.
   Immediate EOF (no bytes) → NULL. */
char *draconic_rt_host_stdin_read_line(void);

/* Read up to max_len bytes. On OK: *out_data is malloc'd of *out_len bytes
   (caller frees with free); *out_len may be 0 at EOF (then *out_data is NULL).
   max_len == 0 → OK with empty result. */
DraconicHostError draconic_rt_host_stdin_read_bytes(
    size_t max_len,
    uint8_t **out_data,
    size_t *out_len);

/* --- Path helpers (H03.01–H03.02): pure string ops; no filesystem I/O ---
   POSIX-style separator `/` in output; input accepts `/` and `\` (Windows-aware).
   Results are malloc'd NUL-terminated UTF-8; free with draconic_rt_host_path_free.
   Empty normalize → "."; empty join → ".". */

/* Normalize `.` / `..` / repeated separators. path may be NULL (= ""). */
char *draconic_rt_host_path_normalize(const char *path);

/* Join n segments then normalize. parts may be NULL when n == 0. */
char *draconic_rt_host_path_join(size_t n, const char *const *parts);

/* Directory name (parent path). Empty → "."; root → "/". */
char *draconic_rt_host_path_dirname(const char *path);

/* Final path segment. Empty / root-only → "". */
char *draconic_rt_host_path_basename(const char *path);

/* Extension including leading `.` ("" if none). */
char *draconic_rt_host_path_extname(const char *path);

/* 1 if path starts with `/` or `\`; 0 otherwise (NULL/empty → 0). */
int32_t draconic_rt_host_path_is_absolute(const char *path);

/* --- Filesystem read (H04.01) ----------------------------------------------
   Whole-file read. Missing path → DRACONIC_HOST_E_NOENT. Path NULL/empty →
   DRACONIC_HOST_E_INVAL. Caller frees out buffers with free() (or
   draconic_rt_host_bytes_storage_free / draconic_rt_host_path_free). */

/* Read entire file as raw bytes. On OK: *out_data is malloc'd of *out_len
   (empty file → *out_data NULL, *out_len 0). */
DraconicHostError draconic_rt_host_fs_read_file(
    const char *path,
    uint8_t **out_data,
    size_t *out_len);

/* Read entire file as UTF-8 text. On OK: *out_text is malloc'd NUL-terminated
   (empty → empty string). Rejects invalid UTF-8 with DRACONIC_HOST_E_INVAL. */
DraconicHostError draconic_rt_host_fs_read_text(
    const char *path,
    char **out_text);

/* --- Filesystem write / append (H04.02) ------------------------------------
   Create parent is not required (ENOENT if missing parent). Create file if
   missing. write truncates; append extends. Path NULL/empty → INVAL.
   data may be NULL only when len == 0. */

/* Write entire file as raw bytes (create/truncate). */
DraconicHostError draconic_rt_host_fs_write_file(
    const char *path,
    const uint8_t *data,
    size_t len);

/* Append raw bytes (create if missing). */
DraconicHostError draconic_rt_host_fs_append_file(
    const char *path,
    const uint8_t *data,
    size_t len);

/* Write entire file as UTF-8 text (NUL-terminated; create/truncate). */
DraconicHostError draconic_rt_host_fs_write_text(
    const char *path,
    const char *text);

/* Append UTF-8 text (NUL-terminated; create if missing). */
DraconicHostError draconic_rt_host_fs_append_text(
    const char *path,
    const char *text);

/* --- Filesystem exists / stat (H04.03) -------------------------------------
   exists: 1 if path exists, 0 if missing or path NULL/empty (no throw).
   stat: missing → DRACONIC_HOST_E_NOENT. size = bytes; is_file/is_dir are
   0/1; mtime_ms is modification time as milliseconds since Unix epoch. */

int32_t draconic_rt_host_fs_exists(const char *path);

DraconicHostError draconic_rt_host_fs_stat(
    const char *path,
    int64_t *out_size,
    int32_t *out_is_file,
    int32_t *out_is_dir,
    double *out_mtime_ms);

/* --- Filesystem directory ops (H04.04) -------------------------------------
   mkdir: create one directory (non-recursive). EEXIST if path already exists.
   mkdir_all: create path and parents (like mkdir -p); OK if path is already a dir.
   readdir: entry names only (no "." / ".."). On OK: *out_names is malloc'd array of
   *out_count malloc'd NUL-terminated names (caller frees each name + the array).
   rmdir: remove empty directory. remove_file: unlink a regular file.
   Missing path → NOENT; path NULL/empty → INVAL. */

DraconicHostError draconic_rt_host_fs_mkdir(const char *path);
DraconicHostError draconic_rt_host_fs_mkdir_all(const char *path);
DraconicHostError draconic_rt_host_fs_readdir(
    const char *path,
    char ***out_names,
    int64_t *out_count);
DraconicHostError draconic_rt_host_fs_rmdir(const char *path);
DraconicHostError draconic_rt_host_fs_remove_file(const char *path);

/* --- Filesystem rename / copy (H04.05) -------------------------------------
   rename_file: rename/move path from → to (same as POSIX rename).
   copy_file: copy regular file contents from → to (overwrite dest if present).
   Missing source → NOENT; path NULL/empty → INVAL. */

DraconicHostError draconic_rt_host_fs_rename_file(const char *from, const char *to);
DraconicHostError draconic_rt_host_fs_copy_file(const char *from, const char *to);

/* --- Open handle (H04.06) --------------------------------------------------
    open: path + mode string "r"|"w"|"a"|"r+"|"w+"|"a+" → live file handle.
    handle_read: up to max_len bytes from current offset (EOF → empty).
    handle_write: write all bytes at current offset (append modes honor O_APPEND).
    handle_seek: whence 0=SET, 1=CUR, 2=END; *out_pos new absolute offset.
    close: draconic_rt_host_handle_close (closes OS fd). */

DraconicHostError draconic_rt_host_fs_open(
    const char *path,
    const char *mode,
    DraconicHostHandle *out_h);
DraconicHostError draconic_rt_host_fs_handle_read(
    DraconicHostHandle h,
    size_t max_len,
    uint8_t **out_data,
    size_t *out_len);
DraconicHostError draconic_rt_host_fs_handle_write(
    DraconicHostHandle h,
    const uint8_t *data,
    size_t len);
DraconicHostError draconic_rt_host_fs_handle_seek(
    DraconicHostHandle h,
    int64_t offset,
    int32_t whence,
    int64_t *out_pos);

/* --- TCP listen/accept/connect/io (H06.01–H06.04) --------------------------
   Bind IPv4 TCP listener. port 0 → OS ephemeral; backlog <= 0 → 128.
   On OK: *out_h is a live listen handle (close with handle_close).
   tcp_local_port: getsockname bound port (1..65535).
   accept: blocking accept → connection handle; peer via peer_address/port.
    connect: dial host:port → connection handle. Host may be IPv4 dotted or a
    DNS name (H09.02 resolve then connect). Resolve failure → E_ADDR.
    Connection refused / reset / unreachable / ETIMEDOUT → DRACONIC_HOST_E_CONN.
   peer_address: malloc'd dotted IPv4 (free with path_free).
   tcp_read: up to max_len bytes (partial OK); peer close → empty (len 0).
   tcp_write: write all bytes (loop); empty len OK.
   tcp_shutdown: how 0=RD, 1=WR, 2=RDWR (POSIX SHUT_*); default use 1=WR. */

DraconicHostError draconic_rt_host_tcp_listen(
    int32_t port,
    int32_t backlog,
    DraconicHostHandle *out_h);
DraconicHostError draconic_rt_host_tcp_local_port(
    DraconicHostHandle h,
    int32_t *out_port);
DraconicHostError draconic_rt_host_tcp_accept(
    DraconicHostHandle listen_h,
    DraconicHostHandle *out_conn);
DraconicHostError draconic_rt_host_tcp_connect(
    const char *host,
    int32_t port,
    DraconicHostHandle *out_conn);
DraconicHostError draconic_rt_host_tcp_peer_port(
    DraconicHostHandle conn_h,
    int32_t *out_port);
DraconicHostError draconic_rt_host_tcp_peer_address(
    DraconicHostHandle conn_h,
    char **out_addr);
DraconicHostError draconic_rt_host_tcp_read(
    DraconicHostHandle conn_h,
    size_t max_len,
    uint8_t **out_data,
    size_t *out_len);
DraconicHostError draconic_rt_host_tcp_write(
    DraconicHostHandle conn_h,
    const uint8_t *data,
    size_t len);
DraconicHostError draconic_rt_host_tcp_shutdown(
    DraconicHostHandle conn_h,
    int32_t how);

/* --- Async socket readiness (H07.01) ---------------------------------------
   Non-blocking mode on TCP listen/conn handles. One-shot readiness waits
   complete by enqueuing a host job (draconic_rt_job_enqueue) when the fd is
   ready; job_drain polls waits (with timer-aware timeout).
   events: bitwise OR of DRACONIC_HOST_IO_READ / DRACONIC_HOST_IO_WRITE.
   READ = readable or accept-ready; WRITE = writable (or connect complete).
   io_wait: register one-shot; *out_id > 0. io_cancel by id. Close cancels
   waits on that handle. io_poll: promote ready waits → jobs; timeout_ms < 0
   blocks until ≥1 ready (or no waits); 0 = non-blocking. io_pending: 1 if
   any live wait remains. */

#define DRACONIC_HOST_IO_READ 1
#define DRACONIC_HOST_IO_WRITE 2

typedef void (*DraconicHostIoFn)(void *data);

DraconicHostError draconic_rt_host_tcp_set_nonblocking(
    DraconicHostHandle h,
    int32_t enable);
DraconicHostError draconic_rt_host_io_wait(
    DraconicHostHandle h,
    int32_t events,
    DraconicHostIoFn fn,
    void *data,
    int64_t *out_id);
void draconic_rt_host_io_cancel(int64_t id);
int draconic_rt_host_io_pending(void);
int draconic_rt_host_io_poll(double timeout_ms);

/* --- Async TCP → Promises (H07.02) -----------------------------------------
   Returns a pending Promise (DraconicValue*). Fulfillment values are opaque
   void* payloads matching the Promise ABI (numbers as inttoptr i64):
     accept/connect → connection handle (i64)
     read           → bytes read (i64); data is not retained
     write          → bytes written (i64)
   Rejection reason is host error code as inttoptr.
   Close of the listen/conn handle cancels waits and rejects pending ops. */
typedef struct DraconicValue DraconicValue;

DraconicValue *draconic_rt_host_tcp_accept_async(DraconicHostHandle listen_h);
DraconicValue *draconic_rt_host_tcp_connect_async(const char *host, int32_t port);
DraconicValue *draconic_rt_host_tcp_read_async(
    DraconicHostHandle conn_h,
    int64_t max_len);
DraconicValue *draconic_rt_host_tcp_write_async(
    DraconicHostHandle conn_h,
    const uint8_t *data,
    size_t len);

/* --- UDP bind/sendto/recvfrom (H08.01) -------------------------------------
   Bind IPv4 UDP socket. port 0 → OS ephemeral.
   On OK: *out_h is a live UDP handle (close with handle_close).
   udp_local_port: getsockname bound port (1..65535).
   udp_sendto: send datagram to IPv4 dotted host:port (all bytes one sendto).
   udp_recvfrom: blocking recv up to max_len; *out_data malloc'd (caller frees
   via free / path_free pattern); peer via optional out_peer_* (NULL skips).
   peer_address: malloc'd dotted IPv4 when requested. */

DraconicHostError draconic_rt_host_udp_bind(
    int32_t port,
    DraconicHostHandle *out_h);
DraconicHostError draconic_rt_host_udp_local_port(
    DraconicHostHandle h,
    int32_t *out_port);
DraconicHostError draconic_rt_host_udp_sendto(
    DraconicHostHandle h,
    const uint8_t *data,
    size_t len,
    const char *host,
    int32_t port);
DraconicHostError draconic_rt_host_udp_recvfrom(
    DraconicHostHandle h,
    size_t max_len,
    uint8_t **out_data,
    size_t *out_len,
    char **out_peer_addr,
    int32_t *out_peer_port);

/* --- DNS lookup (H09.01) ---------------------------------------------------
   Resolve hostname (or IPv4 dotted literal) → list of IPv4 dotted addresses.
   On OK: *out_addrs is malloc'd array of *out_count malloc'd NUL-terminated
   address strings (caller frees each string + the array). Empty / NULL host
   → INVAL. Resolution failure (unknown host, no A records) → E_ADDR.
   Duplicates may be collapsed. v1: AF_INET only. */

DraconicHostError draconic_rt_host_dns_lookup(
    const char *hostname,
    char ***out_addrs,
    int64_t *out_count);

/* --- HTTP/1.1 request parse (H10.01) ---------------------------------------
   Parse a complete plaintext HTTP/1.1 request message (request-line + headers
   + optional body). On OK: *out_method / *out_path / *out_version / *out_body
   are malloc'd NUL-terminated strings (caller frees each with path_free).
   Body is bounded by Content-Length when present (take min(CL, available after
   header block)); without Content-Length, body is empty (chunked → H10.06).
   Malformed request-line, missing header terminator, or bad Content-Length →
   INVAL. Header lookup is case-insensitive; missing name → *out_value is
   malloc'd empty string (always non-NULL on OK). */

DraconicHostError draconic_rt_host_http_parse_request(
    const uint8_t *data,
    size_t len,
    char **out_method,
    char **out_path,
    char **out_version,
    char **out_body);

DraconicHostError draconic_rt_host_http_request_header(
    const uint8_t *data,
    size_t len,
    const char *name,
    char **out_value);

/* --- HTTP/1.1 response write (H10.02) --------------------------------------
   Format a complete plaintext HTTP/1.1 response message (status-line + headers
   + body). status must be 100..599. reason may be empty → default phrase for
   common codes (else empty reason-phrase). headers is optional extra header
   lines ("Name: value\r\n"…); Content-Length is auto-appended when absent
   (case-insensitive). body may be NULL when body_len==0.
   On OK: *out_msg is malloc'd NUL-terminated wire bytes (caller frees). */

DraconicHostError draconic_rt_host_http_write_response(
    int32_t status,
    const char *reason,
    const char *headers,
    const uint8_t *body,
    size_t body_len,
    char **out_msg);

/* --- HTTP/1.1 client helpers (H10.05) --------------------------------------
   write_request: format a complete plaintext HTTP/1.1 request message
   (request-line + headers + body). method/path required non-empty; headers
   optional extra lines ("Name: value\r\n"…); Content-Length auto-appended when
   absent. body may be NULL when body_len==0. On OK: *out_msg malloc'd wire
   bytes (caller frees).

   parse_response: parse a complete plaintext HTTP/1.1 response (status-line +
   headers + optional body). On OK: *out_version / *out_reason / *out_body are
   malloc'd strings; *out_status is the numeric status code (100..599). Body
   bounded by Content-Length when present (same rules as parse_request).
   Malformed status-line / missing header terminator / bad CL → INVAL.

   response_header: case-insensitive header lookup on a response message
   (same semantics as request_header). */

DraconicHostError draconic_rt_host_http_write_request(
    const char *method,
    const char *path,
    const char *headers,
    const uint8_t *body,
    size_t body_len,
    char **out_msg);

DraconicHostError draconic_rt_host_http_parse_response(
    const uint8_t *data,
    size_t len,
    char **out_version,
    int32_t *out_status,
    char **out_reason,
    char **out_body);

DraconicHostError draconic_rt_host_http_response_header(
    const uint8_t *data,
    size_t len,
    const char *name,
    char **out_value);

/* --- WebSocket handshake response (H12.01) ---------------------------------
   RFC 6455 server opening handshake response from Sec-WebSocket-Key.
   On OK: *out_msg is malloc'd wire bytes for:
     HTTP/1.1 101 Switching Protocols
     Upgrade: websocket
     Connection: Upgrade
     Sec-WebSocket-Accept: base64(SHA1(key + GUID))
   Empty/NULL key → INVAL. Caller frees *out_msg. */
DraconicHostError draconic_rt_host_ws_handshake_response(
    const char *sec_websocket_key,
    char **out_msg);

/* --- WebSocket frames (H12.02 / RFC 6455 §5) --------------------------------
   Server frames are unmasked (MASK=0). FIN=1 single-frame messages.
   Opcodes: 1 text, 2 binary, 8 close, 9 ping, 10 pong.
   Encode: *out_data malloc'd of *out_len bytes (caller frees).
   Decode: unmasks client frames; *out_payload malloc'd (caller frees);
   *out_close_code is status code when opcode==8 and payload >= 2, else -1.
   Malformed / truncated → INVAL. */
DraconicHostError draconic_rt_host_ws_encode_text(
    const char *payload,
    uint8_t **out_data,
    size_t *out_len);
DraconicHostError draconic_rt_host_ws_encode_binary(
    const uint8_t *payload,
    size_t payload_len,
    uint8_t **out_data,
    size_t *out_len);
DraconicHostError draconic_rt_host_ws_encode_close(
    int32_t code,
    const char *reason,
    uint8_t **out_data,
    size_t *out_len);
DraconicHostError draconic_rt_host_ws_encode_ping(
    const char *payload,
    uint8_t **out_data,
    size_t *out_len);
DraconicHostError draconic_rt_host_ws_encode_pong(
    const char *payload,
    uint8_t **out_data,
    size_t *out_len);
DraconicHostError draconic_rt_host_ws_decode_frame(
    const uint8_t *data,
    size_t len,
    int32_t *out_fin,
    int32_t *out_opcode,
    uint8_t **out_payload,
    size_t *out_payload_len,
    int32_t *out_close_code);

/* --- WebSocket client dial (H12.03 / RFC 6455) -----------------------------
   Client opening handshake request from path/host/Sec-WebSocket-Key.
   On OK: *out_msg is malloc'd wire bytes (GET … Upgrade). Empty/NULL path,
   host, or key → INVAL. Caller frees *out_msg.
   Check Accept: parse HTTP response; require 101 + matching Accept for key.
   Encode text client: FIN=1 text frame with MASK=1 and random 4-byte mask. */
DraconicHostError draconic_rt_host_ws_client_handshake_request(
    const char *path,
    const char *host,
    const char *sec_websocket_key,
    char **out_msg);
DraconicHostError draconic_rt_host_ws_client_check_accept(
    const uint8_t *data,
    size_t len,
    const char *sec_websocket_key);
DraconicHostError draconic_rt_host_ws_encode_text_client(
    const char *payload,
    uint8_t **out_data,
    size_t *out_len);

/* --- TLS client/server wrap (H11.01 / H11.02) ------------------------------
   Wrap an existing TCP connection handle as a TLS client or server session.
   Takes ownership of the TCP conn handle (it becomes invalid); *out_tls is a
   new TLS handle closed with handle_close / closeTls.
   Client (H11.01):
     server_name: SNI + peer domain for cert verify (may be empty when insecure).
     insecure != 0: skip certificate verification (test mode).
     insecure == 0: system trust roots + hostname check (macOS Secure Transport).
   Server (H11.02):
     cert_path / key_path: PEM files (certificate + matching private key).
   tls_read / tls_write: application data over the TLS session (same shapes as
   tcp_read / tcp_write). */
DraconicHostError draconic_rt_host_tls_client_wrap(
    DraconicHostHandle tcp_conn,
    const char *server_name,
    int32_t insecure,
    DraconicHostHandle *out_tls);
DraconicHostError draconic_rt_host_tls_server_wrap(
    DraconicHostHandle tcp_conn,
    const char *cert_path,
    const char *key_path,
    DraconicHostHandle *out_tls);
DraconicHostError draconic_rt_host_tls_read(
    DraconicHostHandle tls_h,
    size_t max_len,
    uint8_t **out_data,
    size_t *out_len);
DraconicHostError draconic_rt_host_tls_write(
    DraconicHostHandle tls_h,
    const uint8_t *data,
    size_t len);

#ifdef __cplusplus
}
#endif

#endif /* DRACONIC_RT_HOST_H */
