//! ROADMAP F07.01: parse C header subset (scalar/pointer function decls).

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use draconic_cli::c_header::{parse_header, CType, Param};

fn temp_dir() -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "draconic-integration-f07-01-{}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn parse_header_file_scalar_and_pointer_fns() {
    let dir = temp_dir();
    let path = dir.join("api.h");
    fs::write(
        &path,
        r#"
            int add(int a, int b);
            void free(void *p);
            char *strdup(const char *s);
        "#,
    )
    .unwrap();
    let src = fs::read_to_string(&path).unwrap();
    let h = parse_header(&src).expect("parse_header");
    assert_eq!(h.functions.len(), 3);
    assert_eq!(h.functions[0].name, "add");
    assert_eq!(h.functions[0].return_ty, CType::Int);
    assert_eq!(
        h.functions[1].params,
        vec![Param {
            name: Some("p".into()),
            ty: CType::Pointer(Box::new(CType::Void)),
        }]
    );
    assert_eq!(
        h.functions[2].return_ty,
        CType::Pointer(Box::new(CType::Char))
    );
    let _ = fs::remove_dir_all(&dir);
}
