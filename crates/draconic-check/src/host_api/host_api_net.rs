//! Time, timer, TCP, UDP, DNS, HTTP, TLS, WebSocket, and HTTP/2 host API
//! registry rows (H05–H13, H17).

use super::{HostApiEntry, HostAvailability};

pub(super) const ENTRIES: &[HostApiEntry] = &[
    HostApiEntry {
        name: "nowMs",
        availability: HostAvailability::BOTH,
        note: "H05.01 wall clock ms",
    },
    HostApiEntry {
        name: "monotonicMs",
        availability: HostAvailability::BOTH,
        note: "H05.02 monotonic clock ms",
    },
    HostApiEntry {
        name: "setTimeout",
        availability: HostAvailability::BOTH,
        note: "H05.03 setTimeout via job queue",
    },
    HostApiEntry {
        name: "clearTimeout",
        availability: HostAvailability::BOTH,
        note: "H05.03 clearTimeout",
    },
    HostApiEntry {
        name: "setInterval",
        availability: HostAvailability::BOTH,
        note: "H05.04 setInterval via job queue",
    },
    HostApiEntry {
        name: "clearInterval",
        availability: HostAvailability::BOTH,
        note: "H05.04 clearInterval",
    },
    HostApiEntry {
        name: "tlsClientWrap",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H11.01 TLS client wrap TCP conn (serverName, insecure)",
    },
    HostApiEntry {
        name: "tlsServerWrap",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H11.02 TLS server wrap TCP conn (certPath, keyPath PEM)",
    },
    HostApiEntry {
        name: "tlsRead",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H11.01 TLS read application data",
    },
    HostApiEntry {
        name: "tlsWrite",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H11.01 TLS write application data",
    },
    HostApiEntry {
        name: "closeTls",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H11.01 close TLS handle (and underlying TCP)",
    },
    HostApiEntry {
        name: "tcpListen",
        availability: HostAvailability::BOTH,
        note: "H06.01/H17.04 TCP listen",
    },
    HostApiEntry {
        name: "tcpLocalPort",
        availability: HostAvailability::BOTH,
        note: "H06.01/H17.04 TCP local port",
    },
    HostApiEntry {
        name: "closeTcp",
        availability: HostAvailability::BOTH,
        note: "H06.01/H17.04 close TCP listen/conn handle",
    },
    HostApiEntry {
        name: "tcpAccept",
        availability: HostAvailability::BOTH,
        note: "H06.02/H17.04 TCP accept → connection handle",
    },
    HostApiEntry {
        name: "tcpConnect",
        availability: HostAvailability::BOTH,
        note: "H06.03/H17.04 TCP connect dial host:port; refused/timeout → ECONN",
    },
    HostApiEntry {
        name: "tcpPeerAddress",
        availability: HostAvailability::BOTH,
        note: "H06.02/H17.04 TCP peer IPv4 address string",
    },
    HostApiEntry {
        name: "tcpPeerPort",
        availability: HostAvailability::BOTH,
        note: "H06.02/H17.04 TCP peer port",
    },
    HostApiEntry {
        name: "tcpRead",
        availability: HostAvailability::BOTH,
        note: "H06.04/H17.04 TCP read bytes (partial OK)",
    },
    HostApiEntry {
        name: "tcpWrite",
        availability: HostAvailability::BOTH,
        note: "H06.04/H17.04 TCP write bytes",
    },
    HostApiEntry {
        name: "tcpShutdown",
        availability: HostAvailability::BOTH,
        note: "H06.04/H17.04 TCP shutdown (0=RD 1=WR 2=RDWR)",
    },
    HostApiEntry {
        name: "tcpAcceptAsync",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H07.02 TCP accept → Promise (handle)",
    },
    HostApiEntry {
        name: "tcpConnectAsync",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H07.02 TCP connect → Promise (handle)",
    },
    HostApiEntry {
        name: "tcpReadAsync",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H07.02 TCP read → Promise (byte count)",
    },
    HostApiEntry {
        name: "tcpWriteAsync",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H07.02 TCP write → Promise (byte count)",
    },
    HostApiEntry {
        name: "udpBind",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H08.01 UDP bind (port 0 → ephemeral)",
    },
    HostApiEntry {
        name: "udpLocalPort",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H08.01 UDP local bound port",
    },
    HostApiEntry {
        name: "udpSendTo",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H08.01 UDP sendto host:port",
    },
    HostApiEntry {
        name: "udpRecvFrom",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H08.01 UDP recvfrom → bytes",
    },
    HostApiEntry {
        name: "closeUdp",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H08.01 close UDP handle",
    },
    HostApiEntry {
        name: "dnsLookup",
        availability: HostAvailability::BOTH,
        note: "H09.01/H17.04 DNS lookup hostname → IPv4 address strings",
    },
    HostApiEntry {
        name: "httpParseRequest",
        availability: HostAvailability::BOTH,
        note: "H10.01/H17.04 HTTP/1.1 request parse → method/path/version/body",
    },
    HostApiEntry {
        name: "httpRequestHeader",
        availability: HostAvailability::BOTH,
        note: "H10.01/H17.04 HTTP/1.1 request header lookup (case-insensitive)",
    },
    HostApiEntry {
        name: "httpWriteResponse",
        availability: HostAvailability::BOTH,
        note: "H10.02/H17.04 HTTP/1.1 response write → status-line + headers + body",
    },
    HostApiEntry {
        name: "httpServeStatic",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H17.03 HTTP/1.1 static file serve on TCP conn under docroot",
    },
    HostApiEntry {
        name: "httpWriteRequest",
        availability: HostAvailability::BOTH,
        note: "H10.05/H17.04 HTTP/1.1 request write → request-line + headers + body",
    },
    HostApiEntry {
        name: "httpParseResponse",
        availability: HostAvailability::BOTH,
        note: "H10.05/H17.04 HTTP/1.1 response parse → version/status/reason/body",
    },
    HostApiEntry {
        name: "httpResponseHeader",
        availability: HostAvailability::BOTH,
        note: "H10.05/H17.04 HTTP/1.1 response header lookup (case-insensitive)",
    },
    HostApiEntry {
        name: "wsHandshakeResponse",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H12.01 WebSocket server opening handshake response from Sec-WebSocket-Key",
    },
    HostApiEntry {
        name: "wsEncodeText",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H12.02 WebSocket text frame encode (FIN=1, unmasked)",
    },
    HostApiEntry {
        name: "wsEncodeBinary",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H12.02 WebSocket binary frame encode (FIN=1, unmasked)",
    },
    HostApiEntry {
        name: "wsEncodeClose",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H12.02 WebSocket close frame encode (code + reason)",
    },
    HostApiEntry {
        name: "wsEncodePing",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H12.02 WebSocket ping frame encode",
    },
    HostApiEntry {
        name: "wsEncodePong",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H12.02 WebSocket pong frame encode",
    },
    HostApiEntry {
        name: "wsDecodeFrame",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H12.02 WebSocket frame decode (unmask client frames)",
    },
    HostApiEntry {
        name: "wsClientHandshakeRequest",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H12.03 WebSocket client opening handshake request (path, host, key)",
    },
    HostApiEntry {
        name: "wsClientCheckAccept",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H12.03 WebSocket client validate 101 + Sec-WebSocket-Accept",
    },
    HostApiEntry {
        name: "wsEncodeTextClient",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H12.03 WebSocket client text frame encode (FIN=1, masked)",
    },
    HostApiEntry {
        name: "http2ClientPreface",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H13.01 HTTP/2 client connection preface (magic + SETTINGS)",
    },
    HostApiEntry {
        name: "http2ServerPreface",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H13.01 HTTP/2 server connection preface (SETTINGS)",
    },
    HostApiEntry {
        name: "http2SettingsAck",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H13.01 HTTP/2 SETTINGS ACK frame",
    },
    HostApiEntry {
        name: "http2EncodeRequest",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H13.01 HTTP/2 single-stream request (HEADERS+DATA stream 1)",
    },
    HostApiEntry {
        name: "http2EncodeResponse",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H13.01 HTTP/2 single-stream response (HEADERS+DATA stream 1)",
    },
    HostApiEntry {
        name: "http2ParseRequest",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H13.01 HTTP/2 parse single-stream request → method/path/body",
    },
    HostApiEntry {
        name: "http2ParseResponse",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H13.01 HTTP/2 parse single-stream response → status/body",
    },
    HostApiEntry {
        name: "http2ClientOpen",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H13.01 HTTP/2 client preface + request in one buffer",
    },
    HostApiEntry {
        name: "http2ServerReply",
        availability: HostAvailability::NATIVE_ONLY,
        note: "H13.01 HTTP/2 server preface + response in one buffer",
    },
];

#[cfg(test)]
mod tests {
    use super::super::{is_available, is_host_api, lookup, unsupported_diagnostic, CompileTarget};
    use crate::check_for_target;
    use draconic_diagnostics::Span;
    use draconic_parser::parse;

    #[test]
    fn registry_lists_now_ms_both() {
        let entry = lookup("nowMs").expect("nowMs registered");
        assert!(entry.availability.js);
        assert!(entry.availability.native);
        assert!(is_available("nowMs", CompileTarget::Js));
        assert!(is_available("nowMs", CompileTarget::Native));
        assert!(unsupported_diagnostic("nowMs", CompileTarget::Js, Span::dummy()).is_none());
    }

    #[test]
    fn registry_lists_monotonic_ms_both() {
        let entry = lookup("monotonicMs").expect("monotonicMs registered");
        assert!(entry.availability.js);
        assert!(entry.availability.native);
        assert!(is_available("monotonicMs", CompileTarget::Js));
        assert!(is_available("monotonicMs", CompileTarget::Native));
        assert!(unsupported_diagnostic("monotonicMs", CompileTarget::Js, Span::dummy()).is_none());
    }

    #[test]
    fn registry_lists_set_timeout_both() {
        for name in ["setTimeout", "clearTimeout"] {
            let entry = lookup(name).unwrap_or_else(|| panic!("{name} registered"));
            assert!(entry.availability.js, "{name}");
            assert!(entry.availability.native, "{name}");
            assert!(is_available(name, CompileTarget::Js), "{name}");
            assert!(is_available(name, CompileTarget::Native), "{name}");
            assert!(
                unsupported_diagnostic(name, CompileTarget::Js, Span::dummy()).is_none(),
                "{name}"
            );
        }
    }

    #[test]
    fn registry_lists_set_interval_both() {
        for name in ["setInterval", "clearInterval"] {
            let entry = lookup(name).unwrap_or_else(|| panic!("{name} registered"));
            assert!(entry.availability.js, "{name}");
            assert!(entry.availability.native, "{name}");
            assert!(is_available(name, CompileTarget::Js), "{name}");
            assert!(is_available(name, CompileTarget::Native), "{name}");
            assert!(
                unsupported_diagnostic(name, CompileTarget::Js, Span::dummy()).is_none(),
                "{name}"
            );
        }
    }

    #[test]
    fn registry_lists_h17_04_bridge_subset_both() {
        for name in [
            "tcpListen",
            "tcpLocalPort",
            "closeTcp",
            "tcpAccept",
            "tcpConnect",
            "tcpPeerAddress",
            "tcpPeerPort",
            "tcpRead",
            "tcpWrite",
            "tcpShutdown",
            "dnsLookup",
            "httpParseRequest",
            "httpRequestHeader",
            "httpWriteResponse",
            "httpWriteRequest",
            "httpParseResponse",
            "httpResponseHeader",
        ] {
            let entry = lookup(name).unwrap_or_else(|| panic!("{name} registered"));
            assert!(entry.availability.js, "{name}");
            assert!(entry.availability.native, "{name}");
            assert!(is_host_api(name), "{name}");
            assert!(is_available(name, CompileTarget::Js), "{name}");
            assert!(is_available(name, CompileTarget::Native), "{name}");
            assert!(
                unsupported_diagnostic(name, CompileTarget::Js, Span::dummy()).is_none(),
                "{name}"
            );
        }
    }

    #[test]
    fn registry_lists_remaining_net_native_only() {
        for name in [
            "tcpAcceptAsync",
            "tcpConnectAsync",
            "tcpReadAsync",
            "tcpWriteAsync",
            "udpBind",
            "udpLocalPort",
            "udpSendTo",
            "udpRecvFrom",
            "closeUdp",
            "tlsClientWrap",
            "tlsServerWrap",
            "tlsRead",
            "tlsWrite",
            "closeTls",
            "wsHandshakeResponse",
            "wsEncodeText",
            "wsEncodeBinary",
            "wsEncodeClose",
            "wsEncodePing",
            "wsEncodePong",
            "wsDecodeFrame",
            "wsClientHandshakeRequest",
            "wsClientCheckAccept",
            "wsEncodeTextClient",
            "http2ClientPreface",
            "http2ServerPreface",
            "http2SettingsAck",
            "http2EncodeRequest",
            "http2EncodeResponse",
            "http2ParseRequest",
            "http2ParseResponse",
            "http2ClientOpen",
            "http2ServerReply",
        ] {
            let entry = lookup(name).unwrap_or_else(|| panic!("{name} registered"));
            assert!(!entry.availability.js, "{name}");
            assert!(entry.availability.native, "{name}");
            assert!(is_host_api(name), "{name}");
            assert!(is_available(name, CompileTarget::Native), "{name}");
            assert!(!is_available(name, CompileTarget::Js), "{name}");
        }
    }

    #[test]
    fn check_for_target_js_allows_free_tcp_listen() {
        let program = parse("tcpListen(8080);").unwrap();
        check_for_target(program, CompileTarget::Js)
            .expect("H17.04 js Node bridge allows tcpListen");
    }

    #[test]
    fn check_for_target_js_allows_free_tcp_accept() {
        let program = parse("tcpAccept(0);").unwrap();
        check_for_target(program, CompileTarget::Js)
            .expect("H17.04 js Node bridge allows tcpAccept");
    }

    #[test]
    fn check_for_target_js_allows_dns_lookup() {
        let program = parse("dnsLookup(\"localhost\");").unwrap();
        check_for_target(program, CompileTarget::Js)
            .expect("H17.04 js Node bridge allows dnsLookup");
    }

    #[test]
    fn check_for_target_js_allows_http_listen_helpers() {
        for src in [
            "httpParseRequest(\"\");",
            "httpRequestHeader(null, \"Host\");",
            "httpWriteResponse(200, \"OK\", \"\", \"\");",
            "httpWriteRequest(\"GET\", \"/\", \"\", \"\");",
            "httpParseResponse(\"\");",
            "httpResponseHeader(null, \"Content-Type\");",
        ] {
            let program = parse(src).unwrap();
            check_for_target(program, CompileTarget::Js)
                .unwrap_or_else(|e| panic!("H17.04 js Node bridge allows {src}: {e}"));
        }
    }

    #[test]
    fn check_for_target_native_allows_http_listen_helpers() {
        for src in [
            "httpParseRequest(\"\");",
            "httpWriteResponse(200, \"OK\", \"\", \"\");",
        ] {
            let program = parse(src).unwrap();
            check_for_target(program, CompileTarget::Native)
                .unwrap_or_else(|e| panic!("native allows {src}: {e}"));
        }
    }

    #[test]
    fn check_for_target_native_allows_free_tcp_listen() {
        let program = parse("tcpListen(8080);").unwrap();
        check_for_target(program, CompileTarget::Native).expect("native allows host API ref");
    }

    #[test]
    fn check_for_target_native_allows_free_tcp_accept() {
        let program = parse("tcpAccept(0);").unwrap();
        check_for_target(program, CompileTarget::Native).expect("native allows host API ref");
    }
}
