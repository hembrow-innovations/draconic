//! ROADMAP U05: formatter idempotence on fixture-style programs (library path).

use draconic_ast::print_program;
use draconic_parser::{parse, parse_module};

fn format_source(source: &str) -> String {
    let program = match parse(source) {
        Ok(p) => p,
        Err(_) => parse_module(source).expect("parse"),
    };
    print_program(&program)
}

fn assert_idempotent(src: &str) {
    let once = format_source(src);
    let twice = format_source(&once);
    assert_eq!(
        once, twice,
        "fmt not idempotent\n--- once ---\n{once}\n--- twice ---\n{twice}"
    );
    // Formatted output must re-parse.
    parse(&once)
        .or_else(|_| parse_module(&once))
        .unwrap_or_else(|e| panic!("formatted source failed to parse: {e}\n{once}"));
}

#[test]
fn fmt_idempotent_basic_bindings() {
    assert_idempotent("let x=1+2;\nconst y  =  3;\nvar z=x+y;\n");
}

#[test]
fn fmt_idempotent_functions_and_control() {
    assert_idempotent(
        r#"
function add(a,b){return a+b;}
if(true){let x=1;}else if(false){let y=2;}else{let z=3;}
while(x){break;}
for(let i=0;i<10;i=i+1){continue;}
"#,
    );
}

#[test]
fn fmt_idempotent_objects_arrays_arrows() {
    assert_idempotent(
        r#"
let o={a:1,b:2,m(){return this.a;}};
let a=[1,2,...b];
let f=x=>x+1;
let g=(a,b)=>{return a+b;};
"#,
    );
}

#[test]
fn fmt_idempotent_classes() {
    assert_idempotent(
        r#"
class Point {
  constructor(x,y){this.x=x;this.y=y;}
  move(dx,dy){this.x=this.x+dx;}
  get len(){return this.x;}
  static origin(){return new Point(0,0);}
}
"#,
    );
}

#[test]
fn fmt_stable_style_two_space_indent() {
    let out = format_source("function f(){if(true){return 1;}}");
    assert!(out.contains("function f() {\n"), "{out}");
    assert!(out.contains("\n  if ("), "{out}");
    assert!(out.contains("\n    return 1;\n"), "{out}");
}

#[test]
fn fmt_binary_spacing() {
    let out = format_source("let x=1+2*3;");
    assert_eq!(out, "let x = 1 + 2 * 3;\n");
}
