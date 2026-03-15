use std::path::Path;

fn format_source(source: &str) -> String {
    let tokens = party::compiler::lexer::lex(source).expect("lex failed");
    let mut parser = party::compiler::parser::Parser::new(tokens, source.to_string());
    let program = parser.parse_program().expect("parse failed");
    party::compiler::formatter::format_program(&program)
}

// ---- Style convention tests ----

#[test]
fn style_2_space_indent() {
    let input = "machine M { var x: int; start state S { entry { x = 0; } } }";
    let output = format_source(input);
    assert!(output.contains("  start state"), "expected 2-space indent, got:\n{output}");
    assert!(output.contains("    entry"), "expected 4-space (2x2) indent for entry");
}

#[test]
fn style_no_space_before_colon_in_params() {
    let input = "event E; machine M { start state S { on E do (x: int) { } } }";
    let output = format_source(input);
    assert!(output.contains("(x: int)"), "no space before colon in params: {output}");
}

#[test]
fn style_no_space_before_colon_in_event_payload() {
    let output = format_source("event E: any;");
    assert_eq!(output.trim(), "event E: any;");
}

#[test]
fn style_blank_line_before_on_handlers() {
    let input = r#"
event E;
machine M { start state S {
entry { }
on E goto S;
} }
"#;
    let output = format_source(input);
    // Blank line between entry and on-handler
    assert!(output.contains("}\n\n    on E"), "expected blank line before on handler:\n{output}");
}

#[test]
fn style_short_handler_inline() {
    let input = "event E; machine M { start state S { on E do (x: any) { } } }";
    let output = format_source(input);
    assert!(output.contains("on E do (x: any) { }"), "short handler should be inline:\n{output}");
}

#[test]
fn style_named_tuple_no_trailing_comma() {
    let input = "machine M { start state S { entry { send this, E, (s = null); } } } event E: any;";
    let output = format_source(input);
    assert!(output.contains("(s = null)"), "no trailing comma for single-field named tuple:\n{output}");
}

// ---- The user's exact expected output ----

#[test]
fn style_user_example() {
    let input = r#"
event ev: any;
machine Main { start state Init {
entry { send this, ev, (s = null); }
on ev do (x: any) { }
} }
"#;
    let output = format_source(input);
    let expected = "\
event ev: any;

machine Main {
  start state Init {
    entry {
      send this, ev, (s = null);
    }

    on ev do (x: any) { }
  }
}
";
    assert_eq!(output, expected, "output doesn't match expected style:\n{output}");
}

// ---- Basic declaration formatting ----

#[test]
fn format_simple_event() {
    assert_eq!(format_source("event   Ping :  machine ;").trim(), "event Ping: machine;");
}

#[test]
fn format_event_no_payload() {
    assert_eq!(format_source("event  Unit ;").trim(), "event Unit;");
}

#[test]
fn format_enum() {
    assert_eq!(format_source("enum  Foo  {  A , B,  C  }").trim(), "enum Foo { A, B, C }");
}

#[test]
fn format_numbered_enum() {
    assert_eq!(format_source("enum  Foo  {  X=1, Y = 0 }").trim(), "enum Foo { X = 1, Y = 0 }");
}

#[test]
fn format_typedef() {
    assert_eq!(format_source("type  MyType = (a:  int ,  b: bool) ;").trim(), "type MyType = (a: int, b: bool);");
}

#[test]
fn format_foreign_type() {
    assert_eq!(format_source("type  tFloat ;").trim(), "type tFloat;");
}

#[test]
fn format_eventset_decl() {
    assert_eq!(format_source("eventset  ES = { E1 , E2 } ;").trim(), "eventset ES = { E1, E2 };");
}

#[test]
fn format_interface_decl() {
    let output = format_source("interface  I1( int )  receives E1, E2 ;");
    assert_eq!(output.trim(), "interface I1(int) receives E1, E2;");
}

#[test]
fn format_global_param() {
    assert_eq!(format_source("param  g1 , g2 : int ;").trim(), "param g1, g2: int;");
}

// ---- Machine / State formatting ----

#[test]
fn format_simple_machine() {
    let input = r#"
event E;
machine  Main {
var  x : int ;
start  state  Init {
entry { x = 0 ; }
on E goto  Init ;
}
}
"#;
    let output = format_source(input);
    let expected = "\
event E;

machine Main {
  var x: int;

  start state Init {
    entry { x = 0; }

    on E goto Init;
  }
}
";
    assert_eq!(output, expected);
}

#[test]
fn format_spec_monitor() {
    let input = "event E; event F;\nspec M observes E, F { start cold state Init { on E do { assert true; } on F goto Done; } state Done { } }";
    let output = format_source(input);
    assert!(output.contains("spec M observes E, F {"));
    assert!(output.contains("start cold state Init {"));
    assert!(output.contains("on F goto Done;"));
}

#[test]
fn format_hot_cold_states() {
    let input = "event E; spec M observes E { start cold state Good { on E goto Bad; } hot state Bad { on E goto Good; } }";
    let output = format_source(input);
    assert!(output.contains("start cold state Good {"));
    assert!(output.contains("hot state Bad {"));
}

#[test]
fn format_defer_ignore() {
    let input = "event E; event F; event G; machine M { start state S { defer E, F; ignore G; } }";
    let output = format_source(input);
    assert!(output.contains("defer E, F;"));
    assert!(output.contains("ignore G;"));
}

#[test]
fn format_entry_with_payload() {
    let output = format_source("event E: int; machine M { start state S { entry (payload: int) { assert payload == 0; } on E goto S; } }");
    assert!(output.contains("entry (payload: int) {"));
}

#[test]
fn format_named_entry_function() {
    let output = format_source("machine M { start state S { entry myFunc; } fun myFunc() { } }");
    assert!(output.contains("entry myFunc;"));
}

#[test]
fn format_exit_handler() {
    let output = format_source("machine M { start state S { exit { } } }");
    assert!(output.contains("exit { }"));
}

#[test]
fn format_on_event_do_named() {
    let output = format_source("event E; machine M { start state S { on E do myHandler; } fun myHandler() { } }");
    assert!(output.contains("on E do myHandler;"));
}

#[test]
fn format_goto_with_named() {
    let output = format_source("event E; machine M { start state S { on E goto T with myFunc; } state T {} fun myFunc() {} }");
    assert!(output.contains("on E goto T with myFunc;"));
}

#[test]
fn format_multiple_events_on_handler() {
    let output = format_source("event E; event F; machine M { start state S { on E, F goto S; } }");
    assert!(output.contains("on E, F goto S;"));
}

#[test]
fn format_machine_receives_sends() {
    let output = format_source("event E; event F; machine M receives E, F; sends E; { start state S {} }");
    assert!(output.contains("receives E, F;"));
    assert!(output.contains("sends E;"));
}

// ---- Function formatting ----

#[test]
fn format_function() {
    let output = format_source("machine M { start state S { } fun  foo ( x : int ,  y: bool ) : int { var  z : int ; z = x + 1; return  z ; } }");
    assert!(output.contains("fun foo(x: int, y: bool): int {"));
    assert!(output.contains("var z: int;"));
    assert!(output.contains("return z;"));
}

#[test]
fn format_function_no_return_type() {
    let output = format_source("machine M { start state S {} fun doStuff(x: int) { } }");
    assert!(output.contains("fun doStuff(x: int) {"));
    assert!(!output.contains("doStuff(x: int):"));
}

#[test]
fn format_function_no_params() {
    let output = format_source("machine M { start state S {} fun doStuff(): int { return 0; } }");
    assert!(output.contains("fun doStuff(): int {"));
}

// ---- Statement formatting ----

#[test]
fn format_send_raise_goto() {
    let output = format_source("event E: int; machine M { start state S { entry { send this, E, 42; raise E, 1; goto S, 10; } } }");
    assert!(output.contains("send this, E, 42;"));
    assert!(output.contains("raise E, 1;"));
    assert!(output.contains("goto S, 10;"));
}

#[test]
fn format_if_while_foreach() {
    let output = format_source("machine M { var s: seq[int]; start state S { entry { if ( true ) { } else { } while ( false ) { } foreach ( x in s ) { } } } }");
    assert!(output.contains("if (true) {"));
    assert!(output.contains("} else {"));
    assert!(output.contains("while (false) {"));
    assert!(output.contains("foreach (x in s) {"));
}

#[test]
fn format_collection_ops() {
    let output = format_source("machine M { var s: seq[int]; var m: map[int,int]; var st: set[int]; start state S { entry { s += (0, 1); st += (5); m[1] = 2; } } }");
    assert!(output.contains("s += (0, 1);"));
    assert!(output.contains("st += (5);"));
    assert!(output.contains("m[1] = 2;"));
}

#[test]
fn format_named_tuple_expr() {
    let output = format_source("machine M { var t: (a: int, b: bool); start state S { entry { t = ( a =  1  , b = true ) ; } } }");
    assert!(output.contains("t = (a = 1, b = true);"));
}

#[test]
fn format_receive() {
    let output = format_source("event E: int; event F; machine M { start state S { entry { receive { case E: ( p : int ) { } case F: { } } } } }");
    assert!(output.contains("receive {"));
    assert!(output.contains("case E : (p: int) { }"));
    assert!(output.contains("case F : { }"));
}

#[test]
fn format_announce() {
    let output = format_source("event E: int; machine M { start state S { entry { announce E, 42; } } }");
    assert!(output.contains("announce E, 42;"));
}

#[test]
fn format_raise_halt() {
    let output = format_source("machine M { start state S { entry { raise halt; } } }");
    assert!(output.contains("raise halt;"));
}

#[test]
fn format_break_continue() {
    let output = format_source("machine M { var s: seq[int]; start state S { entry { while (true) { break; } foreach (x in s) { continue; } } } }");
    assert!(output.contains("break;"));
    assert!(output.contains("continue;"));
}

#[test]
fn format_return_no_value() {
    let output = format_source("machine M { start state S { entry { return; } } }");
    assert!(output.contains("return;"));
}

// ---- Expression formatting ----

#[test]
fn format_nondet_fairnondet() {
    let output = format_source("machine M { start state S { entry { if ($) { } if ($$) { } } } }");
    assert!(output.contains("if ($)"));
    assert!(output.contains("if ($$)"));
}

#[test]
fn format_choose_expr() {
    let output = format_source("machine M { var x: int; start state S { entry { x = choose(); x = choose(10); } } }");
    assert!(output.contains("x = choose();"));
    assert!(output.contains("x = choose(10);"));
}

#[test]
fn format_cast() {
    let output = format_source("machine M { var x: int; var y: float; start state S { entry { y = x as float; } } }");
    assert!(output.contains("y = x as float;"));
}

#[test]
fn format_format_string() {
    let output = format_source(r#"machine M { start state S { entry { print format("val = {0}", 42); } } }"#);
    assert!(output.contains(r#"format("val = {0}", 42)"#));
}

#[test]
fn format_assert_with_message() {
    let output = format_source(r#"machine M { start state S { entry { assert x == 0, "must be zero"; } } }"#);
    assert!(output.contains(r#"assert x == 0, "must be zero";"#));
}

#[test]
fn format_builtin_functions() {
    let output = format_source("machine M { var m: map[int,int]; start state S { entry { var k: seq[int]; var sz: int; k = keys(m); sz = sizeof(m); } } }");
    assert!(output.contains("keys(m)"));
    assert!(output.contains("sizeof(m)"));
}

#[test]
fn format_default_expr() {
    let output = format_source("machine M { var s: seq[int]; start state S { entry { s = default(seq[int]); } } }");
    assert!(output.contains("s = default(seq[int]);"));
}

#[test]
fn format_complex_expression() {
    let output = format_source("machine M { var x: int; start state S { entry { x = (1 + 2) * 3; assert x > 0 && x < 100; } } }");
    assert!(output.contains("(1 + 2) * 3;"));
    assert!(output.contains("x > 0 && x < 100;"));
}

#[test]
fn format_negation() {
    let output = format_source("machine M { var x: int; start state S { entry { x = -1; assert !false; } } }");
    assert!(output.contains("x = -1;"));
    assert!(output.contains("!false;"));
}

#[test]
fn format_nested_lvalue() {
    let output = format_source("machine M { var t: (a: seq[int], b: int); start state S { entry { t.a[0] = 1; t.b = 2; } } }");
    assert!(output.contains("t.a[0] = 1;"));
    assert!(output.contains("t.b = 2;"));
}

#[test]
fn format_in_operator() {
    let output = format_source("machine M { var m: map[int,int]; start state S { entry { assert 1 in m; } } }");
    assert!(output.contains("1 in m;"));
}

// ---- Module system ----

#[test]
fn format_module_test() {
    let output = format_source("event E; machine Main { start state S {} } test MyTest [main = Main]: { Main }; implementation Impl [main = Main]: { Main };");
    assert!(output.contains("test MyTest [main = Main]: { Main };"));
    assert!(output.contains("implementation Impl [main = Main]: { Main };"));
}

#[test]
fn format_module_compose() {
    let output = format_source("event E; machine M { start state S {} } module Mod1 = { M }; implementation Impl [main = M]: (compose Mod1, Mod1);");
    assert!(output.contains("module Mod1 = { M };"));
    assert!(output.contains("compose Mod1, Mod1"));
}

#[test]
fn format_module_bindings() {
    let output = format_source("event E; machine M { start state S {} } module Mod = { M -> I1 , M -> I2 };");
    assert!(output.contains("{ M -> I1, M -> I2 }"));
}

// ---- Structural tests ----

#[test]
fn format_preserves_blank_lines_between_kinds() {
    let input = "event E; event F; machine M { start state S {} } machine N { start state S {} }";
    let output = format_source(input);
    assert!(output.contains("event E;\nevent F;"), "events should be grouped");
    assert!(output.contains("F;\n\nmachine M"), "blank line between events and machine");
    assert!(output.contains("}\n\nmachine N"), "blank line between machines");
}

#[test]
fn format_idempotent_on_testdata() {
    let testdata = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata");
    let file = testdata.join("Integration/Correct/PingPong/PingPong.p");
    let source = std::fs::read_to_string(&file).unwrap();
    let first = format_source(&source);
    let second = format_source(&first);
    assert_eq!(first, second, "Formatter is not idempotent");
}

#[test]
fn format_preserves_semantic_meaning() {
    let input = r#"
event Ping: machine; event Pong;
machine Main { var pongId: machine;
start state Init { entry { pongId = new PONG(); raise Pong; } on Pong goto Done; }
state Done {} }
machine PONG { start state S {} }
"#;
    let formatted = format_source(input);
    let tokens = party::compiler::lexer::lex(&formatted).expect("formatted should lex");
    let mut parser = party::compiler::parser::Parser::new(tokens, formatted.clone());
    parser.parse_program().expect("formatted should parse");
}

#[test]
fn format_all_parseable_testdata() {
    let testdata = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata");
    let mut count = 0;
    let mut critical = Vec::new();

    for entry in walkdir(&testdata) {
        if entry.extension().is_some_and(|e| e == "p") {
            let source = std::fs::read_to_string(&entry).unwrap();
            let tokens = match party::compiler::lexer::lex(&source) {
                Ok(t) => t,
                Err(_) => continue,
            };
            let mut parser = party::compiler::parser::Parser::new(tokens, source.to_string());
            let program = match parser.parse_program() {
                Ok(p) => p,
                Err(_) => continue,
            };
            count += 1;
            let formatted = party::compiler::formatter::format_program(&program);

            // Critical: formatted output must be re-parseable
            let tokens2 = match party::compiler::lexer::lex(&formatted) {
                Ok(t) => t,
                Err(e) => { critical.push(format!("{}: re-lex: {e}", entry.display())); continue; }
            };
            let mut parser2 = party::compiler::parser::Parser::new(tokens2, formatted);
            if let Err(e) = parser2.parse_program() {
                critical.push(format!("{}: re-parse: {e}", entry.display()));
            }
        }
    }

    assert!(count > 300, "Expected 300+ files, got {count}");
    if !critical.is_empty() {
        panic!("{} files produced unparseable output:\n{}", critical.len(), critical.join("\n"));
    }
}

fn walkdir(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    if dir.is_dir() {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() { files.extend(walkdir(&path)); }
            else { files.push(path); }
        }
    }
    files
}
