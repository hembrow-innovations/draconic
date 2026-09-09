use super::{ExtractV1, NamedSpan};

pub(super) fn emit_json(extract: &ExtractV1) -> String {
    let mut s = String::from("{\"version\":1,");
    s.push_str("\"functions\":");
    emit_array(&mut s, &extract.functions);
    s.push_str(",\"classes\":");
    emit_array(&mut s, &extract.classes);
    s.push_str(",\"typeAliases\":");
    emit_array(&mut s, &extract.type_aliases);
    s.push_str(",\"externFunctions\":");
    emit_array(&mut s, &extract.extern_functions);
    s.push_str(",\"methods\":");
    emit_array(&mut s, &extract.methods);
    s.push_str(",\"constructors\":");
    emit_array(&mut s, &extract.constructors);
    s.push_str(",\"accessors\":");
    emit_array(&mut s, &extract.accessors);
    s.push_str(",\"imports\":");
    emit_array(&mut s, &extract.imports);
    s.push_str(",\"exports\":");
    emit_array(&mut s, &extract.exports);
    s.push_str(",\"calls\":");
    emit_array(&mut s, &extract.calls);
    s.push('}');
    s
}

fn emit_array(s: &mut String, items: &[NamedSpan]) {
    s.push('[');
    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push('{');
        s.push_str("\"name\":");
        push_json_string(s, &item.name);
        s.push_str(",\"startLine\":");
        s.push_str(&item.start_line.to_string());
        s.push_str(",\"endLine\":");
        s.push_str(&item.end_line.to_string());
        if let Some(enclosing) = &item.enclosing {
            s.push_str(",\"enclosing\":");
            push_json_string(s, enclosing);
        }
        if let Some(abi) = &item.abi {
            s.push_str(",\"abi\":");
            push_json_string(s, abi);
        }
        if item.native {
            s.push_str(",\"native\":true");
        }
        if item.member {
            s.push_str(",\"member\":true");
        }
        if item.is_static {
            s.push_str(",\"static\":true");
        }
        if let Some(accessor) = item.accessor {
            s.push_str(",\"accessor\":");
            push_json_string(s, accessor);
        }
        s.push('}');
    }
    s.push(']');
}

fn push_json_string(s: &mut String, value: &str) {
    s.push('"');
    for c in value.chars() {
        match c {
            '"' => s.push_str("\\\""),
            '\\' => s.push_str("\\\\"),
            '\n' => s.push_str("\\n"),
            '\r' => s.push_str("\\r"),
            '\t' => s.push_str("\\t"),
            c if (c as u32) < 0x20 => s.push_str(&format!("\\u{:04x}", c as u32)),
            c => s.push(c),
        }
    }
    s.push('"');
}
