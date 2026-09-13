use std::io::{self, BufRead, Write};

pub(crate) fn write_message<W: Write>(out: &mut W, body: &str) -> io::Result<()> {
    write!(out, "Content-Length: {}\r\n\r\n{}", body.len(), body)?;
    out.flush()
}

pub(crate) fn read_message<R: BufRead>(input: &mut R) -> io::Result<Option<String>> {
    let mut content_length: Option<usize> = None;
    loop {
        let mut line = String::new();
        let n = input.read_line(&mut line)?;
        if n == 0 {
            return Ok(None);
        }
        let header = line.trim_end_matches(['\r', '\n']);
        if header.is_empty() {
            break;
        }
        if let Some((name, value)) = header.split_once(':') {
            if name.eq_ignore_ascii_case("content-length") {
                let parsed = value.trim().parse::<usize>().map_err(|e| {
                    io::Error::new(io::ErrorKind::InvalidData, format!("Content-Length: {e}"))
                })?;
                content_length = Some(parsed);
            }
        }
    }
    let Some(len) = content_length else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "LSP message missing Content-Length",
        ));
    };
    let mut buf = vec![0u8; len];
    input.read_exact(&mut buf)?;
    String::from_utf8(buf)
        .map(Some)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn rpc_roundtrip_body() {
        let mut out = Vec::new();
        write_message(&mut out, "{\"ok\":true}").unwrap();
        let mut cur = Cursor::new(out);
        let body = read_message(&mut cur).unwrap().expect("body");
        assert_eq!(body, "{\"ok\":true}");
        assert!(read_message(&mut cur).unwrap().is_none());
    }

    #[test]
    fn rpc_ignores_content_type_header() {
        let raw = b"Content-Length: 2\r\nContent-Type: application/vscode-jsonrpc; charset=utf-8\r\n\r\n{}";
        let mut cur = Cursor::new(&raw[..]);
        let body = read_message(&mut cur).unwrap().expect("body");
        assert_eq!(body, "{}");
    }
}
