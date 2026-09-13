use std::collections::HashMap;
use std::io::{self, BufRead, Write};

use serde_json::{json, Value};

use crate::rpc::{read_message, write_message};
use crate::Analysis;

pub fn serve_stdio() -> io::Result<()> {
    let stdin = io::stdin().lock();
    let stdout = io::stdout();
    let mut stdout = stdout.lock();
    serve(stdin, &mut stdout)
}

pub fn serve<R, W>(mut input: R, output: &mut W) -> io::Result<()>
where
    R: BufRead,
    W: Write,
{
    let mut session = Session::default();
    while let Some(body) = read_message(&mut input)? {
        let msg: Value = serde_json::from_str(&body)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let outgoing = session.handle(&msg);
        for frame in outgoing {
            write_message(output, &frame.to_string())?;
        }
        if session.exit {
            break;
        }
    }
    Ok(())
}

#[derive(Default)]
struct Session {
    docs: HashMap<String, Analysis>,
    exit: bool,
}

impl Session {
    fn handle(&mut self, msg: &Value) -> Vec<Value> {
        let method = match msg.get("method").and_then(Value::as_str) {
            Some(m) => m,
            None => return Vec::new(),
        };
        let id = msg.get("id").cloned();
        match method {
            "initialize" => vec![rpc_result(id, initialize_result())],
            "initialized" => Vec::new(),
            "shutdown" => vec![rpc_result(id, Value::Null)],
            "exit" => {
                self.exit = true;
                Vec::new()
            }
            "textDocument/didOpen" => self.did_open(&msg["params"]),
            "textDocument/didChange" => self.did_change(&msg["params"]),
            "textDocument/hover" => vec![rpc_result(id, self.hover(&msg["params"]))],
            "textDocument/definition" => vec![rpc_result(id, self.definition(&msg["params"]))],
            "textDocument/completion" => vec![rpc_result(id, self.completion(&msg["params"]))],
            other => {
                if id.is_some() {
                    vec![rpc_error(id, -32601, &format!("Method not found: {other}"))]
                } else {
                    Vec::new()
                }
            }
        }
    }

    fn did_open(&mut self, params: &Value) -> Vec<Value> {
        let uri = params["textDocument"]["uri"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let text = params["textDocument"]["text"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let analysis = Analysis::analyze(text);
        let notif = publish_diagnostics(&uri, &analysis);
        self.docs.insert(uri, analysis);
        vec![notif]
    }

    fn did_change(&mut self, params: &Value) -> Vec<Value> {
        let uri = params["textDocument"]["uri"]
            .as_str()
            .unwrap_or("")
            .to_string();
        let text = params["contentChanges"]
            .as_array()
            .and_then(|c| c.first())
            .and_then(|c| c.get("text"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let analysis = Analysis::analyze(text);
        let notif = publish_diagnostics(&uri, &analysis);
        self.docs.insert(uri, analysis);
        vec![notif]
    }

    fn hover(&self, params: &Value) -> Value {
        let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
        let Some(analysis) = self.docs.get(uri) else {
            return Value::Null;
        };
        let line = params["position"]["line"].as_u64().unwrap_or(0) as u32;
        let character = params["position"]["character"].as_u64().unwrap_or(0) as u32;
        let offset =
            analysis.location_to_offset(line.saturating_add(1), character.saturating_add(1));
        match analysis.hover(offset) {
            Some(h) => {
                let start = analysis.offset_to_location(h.span.start.0);
                let end = analysis.offset_to_location(h.span.end.0);
                json!({
                    "contents": { "kind": "plaintext", "value": h.type_string },
                    "range": lsp_range(start.line, start.column, end.line, end.column)
                })
            }
            None => Value::Null,
        }
    }

    fn definition(&self, params: &Value) -> Value {
        let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
        let Some(analysis) = self.docs.get(uri) else {
            return Value::Null;
        };
        let line = params["position"]["line"].as_u64().unwrap_or(0) as u32;
        let character = params["position"]["character"].as_u64().unwrap_or(0) as u32;
        let offset =
            analysis.location_to_offset(line.saturating_add(1), character.saturating_add(1));
        match analysis.goto_definition(offset) {
            Some(def) => {
                let start = analysis.offset_to_location(def.span.start.0);
                let end = analysis.offset_to_location(def.span.end.0);
                json!({
                    "uri": uri,
                    "range": lsp_range(start.line, start.column, end.line, end.column)
                })
            }
            None => Value::Null,
        }
    }

    fn completion(&self, params: &Value) -> Value {
        let uri = params["textDocument"]["uri"].as_str().unwrap_or("");
        let Some(analysis) = self.docs.get(uri) else {
            return json!({ "isIncomplete": false, "items": [] });
        };
        let items: Vec<Value> = analysis
            .completions()
            .into_iter()
            .map(|c| json!({ "label": c.name }))
            .collect();
        json!({ "isIncomplete": false, "items": items })
    }
}

fn initialize_result() -> Value {
    json!({
        "capabilities": {
            "positionEncoding": "utf-8",
            "textDocumentSync": { "openClose": true, "change": 1 },
            "hoverProvider": true,
            "definitionProvider": true,
            "completionProvider": {}
        },
        "serverInfo": { "name": "draconic" }
    })
}

fn publish_diagnostics(uri: &str, analysis: &Analysis) -> Value {
    let diagnostics: Vec<Value> = analysis
        .diagnostics()
        .iter()
        .map(|d| {
            let start = analysis.offset_to_location(d.span.start.0);
            let end = analysis.offset_to_location(d.span.end.0);
            json!({
                "range": lsp_range(start.line, start.column, end.line, end.column),
                "severity": 1,
                "message": d.message
            })
        })
        .collect();
    json!({
        "jsonrpc": "2.0",
        "method": "textDocument/publishDiagnostics",
        "params": { "uri": uri, "diagnostics": diagnostics }
    })
}

fn lsp_range(start_line: u32, start_col: u32, end_line: u32, end_col: u32) -> Value {
    json!({
        "start": {
            "line": start_line.saturating_sub(1),
            "character": start_col.saturating_sub(1)
        },
        "end": {
            "line": end_line.saturating_sub(1),
            "character": end_col.saturating_sub(1)
        }
    })
}

fn rpc_result(id: Option<Value>, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id.unwrap_or(Value::Null), "result": result })
}

fn rpc_error(id: Option<Value>, code: i64, message: &str) -> Value {
    json!({
        "jsonrpc": "2.0",
        "id": id.unwrap_or(Value::Null),
        "error": { "code": code, "message": message }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rpc::read_message;
    use crate::Analysis;
    use serde_json::{json, Value};
    use std::io::Cursor;

    fn encode(value: &Value) -> Vec<u8> {
        let body = value.to_string();
        format!("Content-Length: {}\r\n\r\n{}", body.len(), body).into_bytes()
    }

    fn decode_all(buf: &[u8]) -> Vec<Value> {
        let mut cur = Cursor::new(buf);
        let mut out = Vec::new();
        while let Some(body) = read_message(&mut cur).unwrap() {
            out.push(serde_json::from_str(&body).unwrap());
        }
        out
    }

    fn drive(client: &[Value]) -> Vec<Value> {
        let mut input = Vec::new();
        for m in client {
            input.extend(encode(m));
        }
        let mut output = Vec::new();
        serve(Cursor::new(input), &mut output).unwrap();
        decode_all(&output)
    }

    fn initialize(id: i64) -> Value {
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "initialize",
            "params": { "capabilities": {} }
        })
    }

    fn initialized() -> Value {
        json!({ "jsonrpc": "2.0", "method": "initialized", "params": {} })
    }

    fn shutdown(id: i64) -> Value {
        json!({ "jsonrpc": "2.0", "id": id, "method": "shutdown", "params": null })
    }

    fn exit() -> Value {
        json!({ "jsonrpc": "2.0", "method": "exit" })
    }

    fn did_open(uri: &str, text: &str) -> Value {
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didOpen",
            "params": {
                "textDocument": {
                    "uri": uri,
                    "languageId": "draconic",
                    "version": 1,
                    "text": text
                }
            }
        })
    }

    fn did_change(uri: &str, text: &str) -> Value {
        json!({
            "jsonrpc": "2.0",
            "method": "textDocument/didChange",
            "params": {
                "textDocument": { "uri": uri, "version": 2 },
                "contentChanges": [{ "text": text }]
            }
        })
    }

    fn hover_req(id: i64, uri: &str, line: u32, character: u32) -> Value {
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "textDocument/hover",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character }
            }
        })
    }

    fn definition_req(id: i64, uri: &str, line: u32, character: u32) -> Value {
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "textDocument/definition",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character }
            }
        })
    }

    fn completion_req(id: i64, uri: &str, line: u32, character: u32) -> Value {
        json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "textDocument/completion",
            "params": {
                "textDocument": { "uri": uri },
                "position": { "line": line, "character": character }
            }
        })
    }

    fn completion_labels(result: &Value) -> Vec<String> {
        let items = result
            .as_array()
            .or_else(|| result.get("items").and_then(Value::as_array));
        match items {
            Some(items) => items
                .iter()
                .filter_map(|item| item["label"].as_str().map(str::to_string))
                .collect(),
            None => Vec::new(),
        }
    }

    fn response<'a>(msgs: &'a [Value], id: i64) -> &'a Value {
        msgs.iter()
            .find(|m| m["id"] == id)
            .unwrap_or_else(|| panic!("missing response id={id} in {msgs:?}"))
    }

    fn publish_diagnostics_msgs(msgs: &[Value]) -> Vec<&Value> {
        msgs.iter()
            .filter(|m| m["method"] == "textDocument/publishDiagnostics")
            .collect()
    }

    #[test]
    fn lsp_stdio_publishes_analysis_diagnostic() {
        let src = "let x: number = \"hello\";";
        let expected = Analysis::analyze(src).diagnostics()[0].message.clone();
        let msgs = drive(&[
            initialize(1),
            initialized(),
            did_open("file:///t.drac", src),
            shutdown(2),
            exit(),
        ]);
        let published = publish_diagnostics_msgs(&msgs);
        assert_eq!(
            published.len(),
            1,
            "expected one publishDiagnostics: {msgs:?}"
        );
        let diags = &published[0]["params"]["diagnostics"];
        assert_eq!(diags.as_array().map(|a| a.len()), Some(1));
        assert_eq!(diags[0]["message"].as_str(), Some(expected.as_str()));
    }

    #[test]
    fn lsp_stdio_hover_returns_analysis_type() {
        let src = "let x = 1;";
        let analysis = Analysis::analyze(src);
        assert!(!analysis.has_errors());
        let off = src.find('x').expect("x") as u32;
        let loc = analysis.offset_to_location(off);
        let expected = analysis.hover(off).expect("hover").type_string;
        let msgs = drive(&[
            initialize(1),
            initialized(),
            did_open("file:///t.drac", src),
            hover_req(3, "file:///t.drac", loc.line - 1, loc.column - 1),
            shutdown(2),
            exit(),
        ]);
        let hover = &response(&msgs, 3)["result"];
        assert_eq!(hover["contents"]["value"].as_str(), Some(expected.as_str()));
    }

    #[test]
    fn lsp_stdio_definition_from_use_to_decl() {
        let src = "let answer = 1;\nlet z = answer;";
        let analysis = Analysis::analyze(src);
        assert!(!analysis.has_errors());
        let use_off = {
            let first = src.find("answer").expect("decl");
            src[first + 1..]
                .find("answer")
                .map(|rel| (first + 1 + rel) as u32)
                .expect("use")
        };
        let loc = analysis.offset_to_location(use_off);
        let def = analysis.goto_definition(use_off).expect("goto");
        let start = analysis.offset_to_location(def.span.start.0);
        let end = analysis.offset_to_location(def.span.end.0);
        let msgs = drive(&[
            initialize(1),
            initialized(),
            did_open("file:///t.drac", src),
            definition_req(3, "file:///t.drac", loc.line - 1, loc.column - 1),
            shutdown(2),
            exit(),
        ]);
        let result = &response(&msgs, 3)["result"];
        assert_eq!(result["uri"].as_str(), Some("file:///t.drac"));
        assert_eq!(
            result["range"],
            lsp_range(start.line, start.column, end.line, end.column)
        );
    }

    #[test]
    fn lsp_stdio_definition_none_when_check_failed() {
        let src = "let x: number = \"hello\";";
        let analysis = Analysis::analyze(src);
        assert!(analysis.has_errors());
        let off = src.find('x').expect("x") as u32;
        let loc = analysis.offset_to_location(off);
        assert!(analysis.goto_definition(off).is_none());
        let msgs = drive(&[
            initialize(1),
            initialized(),
            did_open("file:///t.drac", src),
            definition_req(3, "file:///t.drac", loc.line - 1, loc.column - 1),
            shutdown(2),
            exit(),
        ]);
        let reply = response(&msgs, 3);
        assert!(reply.get("error").is_none(), "{reply:?}");
        assert_eq!(reply["result"], Value::Null);
    }

    #[test]
    fn lsp_stdio_completion_returns_analysis_names() {
        let src = "let count = 1;\nfunction add(a, b) { return a + b; }";
        let analysis = Analysis::analyze(src);
        assert!(!analysis.has_errors());
        let mut expected: Vec<String> =
            analysis.completions().into_iter().map(|c| c.name).collect();
        let msgs = drive(&[
            initialize(1),
            initialized(),
            did_open("file:///t.drac", src),
            completion_req(3, "file:///t.drac", 1, 0),
            shutdown(2),
            exit(),
        ]);
        let reply = response(&msgs, 3);
        assert!(reply.get("error").is_none(), "{reply:?}");
        let mut labels = completion_labels(&reply["result"]);
        labels.sort();
        expected.sort();
        assert_eq!(labels, expected);
    }

    #[test]
    fn lsp_stdio_completion_none_when_check_failed() {
        let src = "let x: number = \"hello\";";
        let analysis = Analysis::analyze(src);
        assert!(analysis.has_errors());
        assert!(analysis.completions().is_empty());
        let msgs = drive(&[
            initialize(1),
            initialized(),
            did_open("file:///t.drac", src),
            completion_req(3, "file:///t.drac", 0, 4),
            shutdown(2),
            exit(),
        ]);
        let reply = response(&msgs, 3);
        assert!(reply.get("error").is_none(), "{reply:?}");
        assert!(completion_labels(&reply["result"]).is_empty());
    }

    #[test]
    fn lsp_stdio_change_republishes_analysis_diagnostic() {
        let ok = "let x = 1;";
        let bad = "let x: number = \"hello\";";
        let expected = Analysis::analyze(bad).diagnostics()[0].message.clone();
        let msgs = drive(&[
            initialize(1),
            initialized(),
            did_open("file:///t.drac", ok),
            did_change("file:///t.drac", bad),
            shutdown(2),
            exit(),
        ]);
        let published = publish_diagnostics_msgs(&msgs);
        assert_eq!(published.len(), 2, "open then change: {msgs:?}");
        assert_eq!(
            published[0]["params"]["diagnostics"]
                .as_array()
                .map(|a| a.len()),
            Some(0)
        );
        assert_eq!(
            published[1]["params"]["diagnostics"][0]["message"].as_str(),
            Some(expected.as_str())
        );
    }
}
