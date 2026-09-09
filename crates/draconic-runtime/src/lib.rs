//! Native Runtime: GC + minimal std (N05) + job queue (N06.01) + Promise ABI (N06.02–N06.10)
//! + host I/O substrate (H00.02–H00.03, H01.01 process args); embed later (N07).

pub mod abi;
mod archive;
pub mod collections;
pub mod compression;
pub mod crypto;
pub mod flags;
mod fuzz;
pub mod host_js_bridge;
mod host_js_polyfill;
pub mod logging;
pub mod mime;
pub mod testing;
pub mod url;
pub use abi::*;
pub use archive::{
    apply_runtime_link_flags, build_runtime_static_lib, build_runtime_static_lib_with_lto,
    c_host_runtime_header_path, c_host_runtime_header_source, c_host_runtime_path,
    c_host_runtime_source, c_runtime_header_path, c_runtime_header_source, c_runtime_path,
    c_runtime_source, c_runtime_source_paths, print_hello,
};
pub use collections::collections_js_polyfill;
pub use compression::compression_js_polyfill;
pub use crypto::{
    aead_js_polyfill, hmac_sha256_js_polyfill, random_bytes_js_polyfill, sha256_js_polyfill,
};
pub use flags::{
    flag_help, parse_flags, parse_flags_js_polyfill, parse_flags_typed, FlagSpec, FlagValue,
    OptionKind, ParsedFlags, ParsedTypedFlags, TypedValue,
};
pub use fuzz::fuzz_runtime;
pub use host_js_bridge::{dns_js_polyfill, http_js_polyfill, tcp_js_polyfill};
pub use host_js_polyfill::host_js_polyfill;
pub use logging::create_logger_js_polyfill;
pub use mime::{
    mime_js_polyfill, parse_multipart, serialize_multipart, MimeError, MimeErrorKind, MimePart,
};
pub use testing::describe_it_js_polyfill;
pub use url::{
    parse_query, parse_url, parse_url_js_polyfill, query_js_polyfill, serialize_query, ParsedUrl,
};

#[cfg(test)]
pub(crate) use archive::{test_tempfile_dir, test_which_clang};

#[cfg(test)]
mod abort_policy_tests;
#[cfg(test)]
mod eval_time_budget_tests;
#[cfg(test)]
mod gc_alloc_budget_tests;
#[cfg(test)]
mod gc_collect_tests;
#[cfg(test)]
mod gc_heap_tests;
#[cfg(test)]
mod host_abi_tests;
#[cfg(test)]
mod host_atomics_tests;
#[cfg(test)]
mod host_bytes_tests;
#[cfg(test)]
mod host_cancel_tests;
#[cfg(test)]
mod host_dns_tests;
#[cfg(test)]
mod host_fs_tests;
#[cfg(test)]
mod host_http2_tests;
#[cfg(test)]
mod host_http_tests;
#[cfg(test)]
mod host_https_tests;
#[cfg(test)]
mod host_mutex_tests;
#[cfg(test)]
mod host_once_tests;
#[cfg(test)]
mod host_process_tests;
#[cfg(test)]
mod host_signal_tests;
#[cfg(test)]
mod host_stdio_tests;
#[cfg(test)]
mod host_tcp_tests;
#[cfg(test)]
mod host_time_tests;
#[cfg(test)]
mod host_tls_tests;
#[cfg(test)]
mod host_udp_tests;
#[cfg(test)]
mod host_worker_tests;
#[cfg(test)]
mod host_ws_tests;
#[cfg(test)]
mod job_queue_tests;
#[cfg(test)]
mod promise_combinator_tests;
#[cfg(test)]
mod promise_then_tests;
#[cfg(test)]
mod r01_resource_limits_tests;
#[cfg(test)]
mod timer_tests;
