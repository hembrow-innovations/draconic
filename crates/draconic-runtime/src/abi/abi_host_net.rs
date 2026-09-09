use super::{
    AbiFn, GC_INIT, HOST_HANDLE_CLOSE, JOB_DRAIN, PRINT_BOOL, PRINT_I64, PRINT_STR,
    PROMISE_THEN,
};

/* H11.01 / H11.02: TLS client/server wrap + read/write. */
pub const HOST_TLS_CLIENT_WRAP: AbiFn = AbiFn {
    symbol: "draconic_rt_host_tls_client_wrap",
    ret: "i32",
    params: "i64, ptr, i32, ptr",
};
pub const HOST_TLS_SERVER_WRAP: AbiFn = AbiFn {
    symbol: "draconic_rt_host_tls_server_wrap",
    ret: "i32",
    params: "i64, ptr, ptr, ptr",
};
pub const HOST_TLS_READ: AbiFn = AbiFn {
    symbol: "draconic_rt_host_tls_read",
    ret: "i32",
    params: "i64, i64, ptr, ptr",
};
pub const HOST_TLS_WRITE: AbiFn = AbiFn {
    symbol: "draconic_rt_host_tls_write",
    ret: "i32",
    params: "i64, ptr, i64",
};
/* H06.01–H06.04: TCP listen/accept/connect/peer/read/write/shutdown. */
pub const HOST_TCP_LISTEN: AbiFn = AbiFn {
    symbol: "draconic_rt_host_tcp_listen",
    ret: "i32",
    params: "i32, i32, ptr",
};
pub const HOST_TCP_LOCAL_PORT: AbiFn = AbiFn {
    symbol: "draconic_rt_host_tcp_local_port",
    ret: "i32",
    params: "i64, ptr",
};
pub const HOST_TCP_ACCEPT: AbiFn = AbiFn {
    symbol: "draconic_rt_host_tcp_accept",
    ret: "i32",
    params: "i64, ptr",
};
pub const HOST_TCP_CONNECT: AbiFn = AbiFn {
    symbol: "draconic_rt_host_tcp_connect",
    ret: "i32",
    params: "ptr, i32, ptr",
};
pub const HOST_TCP_PEER_PORT: AbiFn = AbiFn {
    symbol: "draconic_rt_host_tcp_peer_port",
    ret: "i32",
    params: "i64, ptr",
};
pub const HOST_TCP_PEER_ADDRESS: AbiFn = AbiFn {
    symbol: "draconic_rt_host_tcp_peer_address",
    ret: "i32",
    params: "i64, ptr",
};
pub const HOST_TCP_READ: AbiFn = AbiFn {
    symbol: "draconic_rt_host_tcp_read",
    ret: "i32",
    params: "i64, i64, ptr, ptr",
};
pub const HOST_TCP_WRITE: AbiFn = AbiFn {
    symbol: "draconic_rt_host_tcp_write",
    ret: "i32",
    params: "i64, ptr, i64",
};
pub const HOST_TCP_SHUTDOWN: AbiFn = AbiFn {
    symbol: "draconic_rt_host_tcp_shutdown",
    ret: "i32",
    params: "i64, i32",
};
/* H07.01: non-blocking readiness + job-queue completion. */
pub const HOST_TCP_SET_NONBLOCKING: AbiFn = AbiFn {
    symbol: "draconic_rt_host_tcp_set_nonblocking",
    ret: "i32",
    params: "i64, i32",
};
pub const HOST_IO_WAIT: AbiFn = AbiFn {
    symbol: "draconic_rt_host_io_wait",
    ret: "i32",
    params: "i64, i32, ptr, ptr, ptr",
};
pub const HOST_IO_CANCEL: AbiFn = AbiFn {
    symbol: "draconic_rt_host_io_cancel",
    ret: "void",
    params: "i64",
};
pub const HOST_IO_PENDING: AbiFn = AbiFn {
    symbol: "draconic_rt_host_io_pending",
    ret: "i32",
    params: "",
};
pub const HOST_IO_POLL: AbiFn = AbiFn {
    symbol: "draconic_rt_host_io_poll",
    ret: "i32",
    params: "double",
};
/* H08.01: UDP bind/sendto/recvfrom. */
pub const HOST_UDP_BIND: AbiFn = AbiFn {
    symbol: "draconic_rt_host_udp_bind",
    ret: "i32",
    params: "i32, ptr",
};
pub const HOST_UDP_LOCAL_PORT: AbiFn = AbiFn {
    symbol: "draconic_rt_host_udp_local_port",
    ret: "i32",
    params: "i64, ptr",
};
pub const HOST_UDP_SENDTO: AbiFn = AbiFn {
    symbol: "draconic_rt_host_udp_sendto",
    ret: "i32",
    params: "i64, ptr, i64, ptr, i32",
};
pub const HOST_UDP_RECVFROM: AbiFn = AbiFn {
    symbol: "draconic_rt_host_udp_recvfrom",
    ret: "i32",
    params: "i64, i64, ptr, ptr, ptr, ptr",
};
/* H09.01: DNS lookup hostname → IPv4 address strings. */
pub const HOST_DNS_LOOKUP: AbiFn = AbiFn {
    symbol: "draconic_rt_host_dns_lookup",
    ret: "i32",
    params: "ptr, ptr, ptr",
};
/* H10.01: HTTP/1.1 request parse (method/path/version/body + header lookup). */
pub const HOST_HTTP_PARSE_REQUEST: AbiFn = AbiFn {
    symbol: "draconic_rt_host_http_parse_request",
    ret: "i32",
    params: "ptr, i64, ptr, ptr, ptr, ptr",
};
pub const HOST_HTTP_REQUEST_HEADER: AbiFn = AbiFn {
    symbol: "draconic_rt_host_http_request_header",
    ret: "i32",
    params: "ptr, i64, ptr, ptr",
};
/* H10.02: HTTP/1.1 response write (status + reason + headers + body → message). */
pub const HOST_HTTP_WRITE_RESPONSE: AbiFn = AbiFn {
    symbol: "draconic_rt_host_http_write_response",
    ret: "i32",
    params: "i32, ptr, ptr, ptr, i64, ptr",
};
/* H17.03: one-shot static file serve on TCP connection under docroot. */
pub const HOST_HTTP_SERVE_STATIC: AbiFn = AbiFn {
    symbol: "draconic_rt_host_http_serve_static",
    ret: "i32",
    params: "i64, ptr",
};
/* H10.05: HTTP/1.1 client — write request + parse response on connected TCP. */
pub const HOST_HTTP_WRITE_REQUEST: AbiFn = AbiFn {
    symbol: "draconic_rt_host_http_write_request",
    ret: "i32",
    params: "ptr, ptr, ptr, ptr, i64, ptr",
};
pub const HOST_HTTP_PARSE_RESPONSE: AbiFn = AbiFn {
    symbol: "draconic_rt_host_http_parse_response",
    ret: "i32",
    params: "ptr, i64, ptr, ptr, ptr, ptr",
};
pub const HOST_HTTP_RESPONSE_HEADER: AbiFn = AbiFn {
    symbol: "draconic_rt_host_http_response_header",
    ret: "i32",
    params: "ptr, i64, ptr, ptr",
};
/* H12.01: WebSocket server opening handshake response (RFC 6455). */
pub const HOST_WS_HANDSHAKE_RESPONSE: AbiFn = AbiFn {
    symbol: "draconic_rt_host_ws_handshake_response",
    ret: "i32",
    params: "ptr, ptr",
};
/* H12.02: WebSocket frames (RFC 6455 §5). */
pub const HOST_WS_ENCODE_TEXT: AbiFn = AbiFn {
    symbol: "draconic_rt_host_ws_encode_text",
    ret: "i32",
    params: "ptr, ptr, ptr",
};
pub const HOST_WS_ENCODE_BINARY: AbiFn = AbiFn {
    symbol: "draconic_rt_host_ws_encode_binary",
    ret: "i32",
    params: "ptr, i64, ptr, ptr",
};
pub const HOST_WS_ENCODE_CLOSE: AbiFn = AbiFn {
    symbol: "draconic_rt_host_ws_encode_close",
    ret: "i32",
    params: "i32, ptr, ptr, ptr",
};
pub const HOST_WS_ENCODE_PING: AbiFn = AbiFn {
    symbol: "draconic_rt_host_ws_encode_ping",
    ret: "i32",
    params: "ptr, ptr, ptr",
};
pub const HOST_WS_ENCODE_PONG: AbiFn = AbiFn {
    symbol: "draconic_rt_host_ws_encode_pong",
    ret: "i32",
    params: "ptr, ptr, ptr",
};
pub const HOST_WS_DECODE_FRAME: AbiFn = AbiFn {
    symbol: "draconic_rt_host_ws_decode_frame",
    ret: "i32",
    params: "ptr, i64, ptr, ptr, ptr, ptr, ptr",
};
/* H12.03: WebSocket client dial (handshake request, Accept check, masked text). */
pub const HOST_WS_CLIENT_HANDSHAKE_REQUEST: AbiFn = AbiFn {
    symbol: "draconic_rt_host_ws_client_handshake_request",
    ret: "i32",
    params: "ptr, ptr, ptr, ptr",
};
pub const HOST_WS_CLIENT_CHECK_ACCEPT: AbiFn = AbiFn {
    symbol: "draconic_rt_host_ws_client_check_accept",
    ret: "i32",
    params: "ptr, i64, ptr",
};
pub const HOST_WS_ENCODE_TEXT_CLIENT: AbiFn = AbiFn {
    symbol: "draconic_rt_host_ws_encode_text_client",
    ret: "i32",
    params: "ptr, ptr, ptr",
};
/* H13.01: HTTP/2 preface + single-stream request/response. */
pub const HOST_HTTP2_CLIENT_PREFACE: AbiFn = AbiFn {
    symbol: "draconic_rt_host_http2_client_preface",
    ret: "i32",
    params: "ptr, ptr",
};
pub const HOST_HTTP2_SERVER_PREFACE: AbiFn = AbiFn {
    symbol: "draconic_rt_host_http2_server_preface",
    ret: "i32",
    params: "ptr, ptr",
};
pub const HOST_HTTP2_SETTINGS_ACK: AbiFn = AbiFn {
    symbol: "draconic_rt_host_http2_settings_ack",
    ret: "i32",
    params: "ptr, ptr",
};
pub const HOST_HTTP2_ENCODE_REQUEST: AbiFn = AbiFn {
    symbol: "draconic_rt_host_http2_encode_request",
    ret: "i32",
    params: "ptr, ptr, ptr, i64, ptr, ptr",
};
pub const HOST_HTTP2_ENCODE_RESPONSE: AbiFn = AbiFn {
    symbol: "draconic_rt_host_http2_encode_response",
    ret: "i32",
    params: "i32, ptr, i64, ptr, ptr",
};
pub const HOST_HTTP2_PARSE_REQUEST: AbiFn = AbiFn {
    symbol: "draconic_rt_host_http2_parse_request",
    ret: "i32",
    params: "ptr, i64, ptr, ptr, ptr, ptr, ptr",
};
pub const HOST_HTTP2_PARSE_RESPONSE: AbiFn = AbiFn {
    symbol: "draconic_rt_host_http2_parse_response",
    ret: "i32",
    params: "ptr, i64, ptr, ptr, ptr, ptr",
};
pub const HOST_HTTP2_CLIENT_OPEN: AbiFn = AbiFn {
    symbol: "draconic_rt_host_http2_client_open",
    ret: "i32",
    params: "ptr, ptr, ptr, i64, ptr, ptr",
};
pub const HOST_HTTP2_SERVER_REPLY: AbiFn = AbiFn {
    symbol: "draconic_rt_host_http2_server_reply",
    ret: "i32",
    params: "i32, ptr, i64, ptr, ptr",
};
/* H07.02: async TCP → Promise. */
pub const HOST_TCP_ACCEPT_ASYNC: AbiFn = AbiFn {
    symbol: "draconic_rt_host_tcp_accept_async",
    ret: "ptr",
    params: "i64",
};
pub const HOST_TCP_CONNECT_ASYNC: AbiFn = AbiFn {
    symbol: "draconic_rt_host_tcp_connect_async",
    ret: "ptr",
    params: "ptr, i32",
};
pub const HOST_TCP_READ_ASYNC: AbiFn = AbiFn {
    symbol: "draconic_rt_host_tcp_read_async",
    ret: "ptr",
    params: "i64, i64",
};
pub const HOST_TCP_WRITE_ASYNC: AbiFn = AbiFn {
    symbol: "draconic_rt_host_tcp_write_async",
    ret: "ptr",
    params: "i64, ptr, i64",
};

/// Declares for H07.02 async TCP + Promise then + job drain.
pub const HOST_TCP_ASYNC_DECLARES: &[AbiFn] = &[
    GC_INIT,
    JOB_DRAIN,
    PROMISE_THEN,
    PRINT_I64,
    PRINT_STR,
    PRINT_BOOL,
    HOST_HANDLE_CLOSE,
    HOST_TCP_LISTEN,
    HOST_TCP_LOCAL_PORT,
    HOST_TCP_ACCEPT,
    HOST_TCP_CONNECT,
    HOST_TCP_ACCEPT_ASYNC,
    HOST_TCP_CONNECT_ASYNC,
    HOST_TCP_READ_ASYNC,
    HOST_TCP_WRITE_ASYNC,
];

