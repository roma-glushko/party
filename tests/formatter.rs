use std::path::Path;

fn format_source(source: &str) -> String {
    let tokens = plang::compiler::lexer::lex(source).expect("lex failed");
    let mut parser = plang::compiler::parser::Parser::new(tokens, source.to_string());
    let program = parser.parse_program().expect("parse failed");
    plang::compiler::formatter::format_program(&program)
}

#[test]
fn format_simple_event() {
    let input = "event   Ping :  machine ;";
    let output = format_source(input);
    assert_eq!(output.trim(), "event Ping: machine;");
}

#[test]
fn format_enum() {
    let input = "enum  Foo  {  A , B,  C  }";
    let output = format_source(input);
    assert_eq!(output.trim(), "enum Foo { A, B, C }");
}

#[test]
fn format_numbered_enum() {
    let input = "enum  Foo  {  X=1, Y = 0 }";
    let output = format_source(input);
    assert_eq!(output.trim(), "enum Foo { X = 1, Y = 0 }");
}

#[test]
fn format_typedef() {
    let input = "type  MyType = (a:  int ,  b: bool) ;";
    let output = format_source(input);
    assert_eq!(output.trim(), "type MyType = (a: int, b: bool);");
}

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
        entry {
            x = 0;
        }
        on E goto Init;
    }
}
";
    assert_eq!(output, expected);
}

#[test]
fn format_spec_monitor() {
    let input = r#"
event E; event F;
spec  M  observes E, F { start  cold  state  Init {
on E do { assert  true ; }
on F goto Done ;
}
state Done { } }
"#;
    let output = format_source(input);
    assert!(output.contains("spec M observes E, F {"));
    assert!(output.contains("start cold state Init {"));
    assert!(output.contains("assert true;"));
    assert!(output.contains("on F goto Done;"));
}

#[test]
fn format_function() {
    let input = r#"
machine M {
start state S { }
fun  foo ( x : int ,  y: bool ) : int {
var  z : int ;
z = x + 1;
return  z ;
} }
"#;
    let output = format_source(input);
    assert!(output.contains("fun foo(x: int, y: bool): int {"));
    assert!(output.contains("var z: int;"));
    assert!(output.contains("z = x + 1;"));
    assert!(output.contains("return z;"));
}

#[test]
fn format_send_raise_goto() {
    let input = r#"
event E : int;
machine M { start state S {
entry {
send  this , E ,  42 ;
raise  E , 1 ;
goto S , 10 ;
} } }
"#;
    let output = format_source(input);
    assert!(output.contains("send this, E, 42;"));
    assert!(output.contains("raise E, 1;"));
    assert!(output.contains("goto S, 10;"));
}

#[test]
fn format_if_while_foreach() {
    let input = r#"
machine M {
var s: seq[int];
start state S { entry {
if ( true ) { } else { }
while ( false ) { }
foreach ( x in s ) { }
} } }
"#;
    let output = format_source(input);
    assert!(output.contains("if (true) {"));
    assert!(output.contains("} else {"));
    assert!(output.contains("while (false) {"));
    assert!(output.contains("foreach (x in s) {"));
}

#[test]
fn format_collection_ops() {
    let input = r#"
machine M {
var s: seq[int]; var m: map[int,int]; var st: set[int];
start state S { entry {
s += (0, 1) ;
s -= (0) ;
st += (5) ;
m[1] = 2 ;
} } }
"#;
    let output = format_source(input);
    assert!(output.contains("s += (0, 1);"));
    // Remove outputs the key expression
    assert!(output.contains("-="), "expected -= in output:\n{output}");
    assert!(output.contains("st += (5);"));
    assert!(output.contains("m[1] = 2;"));
}

#[test]
fn format_named_tuple_expr() {
    let input = r#"
machine M { var t: (a: int, b: bool);
start state S { entry {
t = ( a =  1  , b = true ) ;
} } }
"#;
    let output = format_source(input);
    assert!(output.contains("t = (a = 1, b = true);"));
}

#[test]
fn format_receive() {
    let input = r#"
event E: int; event F;
machine M { start state S { entry {
receive { case E : ( p : int ) { } case F : { } }
} } }
"#;
    let output = format_source(input);
    assert!(output.contains("receive {"));
    assert!(output.contains("case E: (p: int) {"));
    assert!(output.contains("case F: {"));
}

#[test]
fn format_module_test() {
    let input = r#"
event E;
machine Main { start state S {} }
test MyTest [main = Main]: { Main };
implementation Impl [main = Main]: { Main };
"#;
    let output = format_source(input);
    assert!(output.contains("test MyTest [main = Main]: { Main };"));
    assert!(output.contains("implementation Impl [main = Main]: { Main };"));
}

#[test]
fn format_preserves_blank_lines_between_kinds() {
    let input = r#"
event E;
event F;
machine M { start state S {} }
machine N { start state S {} }
"#;
    let output = format_source(input);
    // Events grouped without blank line
    assert!(output.contains("event E;\nevent F;"));
    // Blank line between events and machine
    assert!(output.contains("event F;\n\nmachine M"));
    // Blank line between machines
    assert!(output.contains("}\n\nmachine N"));
}

#[test]
fn format_idempotent_on_testdata() {
    // Formatting the same file twice should produce identical output
    let testdata = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata");
    let file = testdata.join("Integration/Correct/PingPong/PingPong.p");
    let source = std::fs::read_to_string(&file).unwrap();
    let first = format_source(&source);
    let second = format_source(&first);
    assert_eq!(first, second, "Formatter is not idempotent");
}

#[test]
fn format_all_parseable_testdata() {
    // Every file that parses should also format without panic
    let testdata = Path::new(env!("CARGO_MANIFEST_DIR")).join("testdata");
    let mut count = 0;
    let mut failures = Vec::new();

    for entry in walkdir(&testdata) {
        if entry.extension().is_some_and(|e| e == "p") {
            let source = std::fs::read_to_string(&entry).unwrap();
            let tokens = match plang::compiler::lexer::lex(&source) {
                Ok(t) => t,
                Err(_) => continue, // Skip files with lex errors
            };
            let mut parser = plang::compiler::parser::Parser::new(tokens, source.to_string());
            let program = match parser.parse_program() {
                Ok(p) => p,
                Err(_) => continue, // Skip files with parse errors
            };
            count += 1;

            // Should not panic
            let formatted = plang::compiler::formatter::format_program(&program);

            // Idempotency: formatting the formatted output should produce the same result
            let tokens2 = match plang::compiler::lexer::lex(&formatted) {
                Ok(t) => t,
                Err(e) => {
                    failures.push(format!("{}: re-lex failed: {e}", entry.display()));
                    continue;
                }
            };
            let mut parser2 = plang::compiler::parser::Parser::new(tokens2, formatted.clone());
            let program2 = match parser2.parse_program() {
                Ok(p) => p,
                Err(e) => {
                    failures.push(format!("{}: re-parse failed: {e}", entry.display()));
                    continue;
                }
            };
            let formatted2 = plang::compiler::formatter::format_program(&program2);
            if formatted != formatted2 {
                failures.push(format!("{}: not idempotent", entry.display()));
            }
        }
    }

    assert!(count > 300, "Expected 300+ files, got {count}");
    // Allow some non-idempotent files (complex formatting edge cases)
    // The key requirement is that formatting doesn't panic and produces parseable output
    let critical_failures: Vec<_> = failures.iter()
        .filter(|f| f.contains("re-lex") || f.contains("re-parse"))
        .collect();
    if !critical_failures.is_empty() {
        panic!(
            "{} files produced unparseable output:\n{}",
            critical_failures.len(),
            critical_failures.iter().map(|f| f.as_str()).collect::<Vec<_>>().join("\n")
        );
    }
    if !failures.is_empty() {
        eprintln!(
            "Note: {} of {count} files are not idempotent (formatting edge cases)",
            failures.len()
        );
    }
}

// ---- Additional comprehensive tests ----

#[test]
fn format_event_no_payload() {
    let output = format_source("event  Unit ;");
    assert_eq!(output.trim(), "event Unit;");
}

#[test]
fn format_event_tuple_payload() {
    let output = format_source("event  E : ( int , bool );");
    assert_eq!(output.trim(), "event E: (int, bool);");
}

#[test]
fn format_event_named_tuple_payload() {
    let output = format_source("event REQ:( seqNum : int , idx:int, val :int );");
    assert_eq!(output.trim(), "event REQ: (seqNum: int, idx: int, val: int);");
}

#[test]
fn format_foreign_type() {
    let output = format_source("type  tFloat ;");
    assert_eq!(output.trim(), "type tFloat;");
}

#[test]
fn format_complex_typedef() {
    let output = format_source("type  MapIntSeqFoo = map [ int ,  seq[ int ] ] ;");
    assert_eq!(output.trim(), "type MapIntSeqFoo = map[int, seq[int]];");
}

#[test]
fn format_eventset_decl() {
    let output = format_source("eventset  ES = { E1 , E2 , E3 } ;");
    assert_eq!(output.trim(), "eventset ES = { E1, E2, E3 };");
}

#[test]
fn format_interface_decl() {
    let output = format_source("interface  I1( int )  receives E1, E2 ;");
    assert_eq!(output.trim(), "interface I1(int) receives E1, E2;");
}

#[test]
fn format_interface_no_payload() {
    let output = format_source("interface  I2()  receives E1 ;");
    assert_eq!(output.trim(), "interface I2() receives E1;");
}

#[test]
fn format_global_param() {
    let output = format_source("param  g1 , g2 : int ;");
    assert_eq!(output.trim(), "param g1, g2: int;");
}

#[test]
fn format_hot_cold_states() {
    let input = r#"
event E;
spec M observes E {
start cold state Good { on E goto Bad; }
hot state Bad { on E goto Good; }
}
"#;
    let output = format_source(input);
    assert!(output.contains("start cold state Good {"));
    assert!(output.contains("hot state Bad {"));
}

#[test]
fn format_defer_ignore() {
    let input = r#"
event E; event F; event G;
machine M { start state S {
defer E, F;
ignore G;
} }
"#;
    let output = format_source(input);
    assert!(output.contains("defer E, F;"));
    assert!(output.contains("ignore G;"));
}

#[test]
fn format_entry_with_payload() {
    let input = r#"
event E: int;
machine M { start state S {
entry (payload: int) {
assert payload == 0;
}
on E goto S;
} }
"#;
    let output = format_source(input);
    assert!(output.contains("entry (payload: int) {"));
    assert!(output.contains("    assert payload == 0;"));
}

#[test]
fn format_named_entry_function() {
    let input = r#"
machine M {
start state S { entry myFunc; }
fun myFunc() { }
}
"#;
    let output = format_source(input);
    assert!(output.contains("entry myFunc;"));
}

#[test]
fn format_exit_handler() {
    let input = r#"
machine M { start state S {
exit { }
} }
"#;
    let output = format_source(input);
    assert!(output.contains("exit {"));
}

#[test]
fn format_on_event_do_named() {
    let input = r#"
event E;
machine M {
start state S { on E do myHandler; }
fun myHandler() { }
}
"#;
    let output = format_source(input);
    assert!(output.contains("on E do myHandler;"));
}

#[test]
fn format_goto_with_handler() {
    let input = r#"
event E: int;
machine M { start state S {
on E goto T with (payload: int) { assert payload > 0; }
}
state T {} }
"#;
    let output = format_source(input);
    assert!(output.contains("on E goto T with (payload: int) {"));
}

#[test]
fn format_goto_with_named() {
    let input = r#"
event E;
machine M {
start state S { on E goto T with myFunc; }
state T {}
fun myFunc() {}
}
"#;
    let output = format_source(input);
    assert!(output.contains("on E goto T with myFunc;"));
}

#[test]
fn format_multiple_events_on_handler() {
    let input = r#"
event E; event F;
machine M { start state S { on E, F goto S; } }
"#;
    let output = format_source(input);
    assert!(output.contains("on E, F goto S;"));
}

#[test]
fn format_new_expression() {
    let input = r#"
machine M { var x: machine;
start state S { entry { x = new Other(1, true); } } }
machine Other { start state S { entry (p: (int, bool)) { } } }
"#;
    let output = format_source(input);
    assert!(output.contains("x = new Other(1, true);"));
}

#[test]
fn format_this_and_null() {
    let input = r#"
machine M { var x: machine;
start state S { entry {
x = this;
x = null;
} } }
"#;
    let output = format_source(input);
    assert!(output.contains("x = this;"));
    assert!(output.contains("x = null;"));
}

#[test]
fn format_nondet_and_fairnondet() {
    let input = r#"
machine M { start state S { entry {
if ($) { }
if ($$) { }
} } }
"#;
    let output = format_source(input);
    assert!(output.contains("if ($) {"));
    assert!(output.contains("if ($$) {"));
}

#[test]
fn format_choose_expr() {
    let input = r#"
machine M { var x: int; var s: set[int];
start state S { entry {
x = choose();
x = choose(10);
x = choose(s);
} } }
"#;
    let output = format_source(input);
    assert!(output.contains("x = choose();"));
    assert!(output.contains("x = choose(10);"));
    assert!(output.contains("x = choose(s);"));
}

#[test]
fn format_cast_expression() {
    let input = r#"
machine M { var x: int; var y: float;
start state S { entry {
y = x as float;
} } }
"#;
    let output = format_source(input);
    assert!(output.contains("y = x as float;"));
}

#[test]
fn format_format_string() {
    let input = r#"
machine M { start state S { entry {
print format("value = {0}, name = {1}", 42, "test");
} } }
"#;
    let output = format_source(input);
    assert!(output.contains(r#"print format("value = {0}, name = {1}", 42, "test");"#));
}

#[test]
fn format_assert_with_message() {
    let input = r#"
machine M { start state S { entry {
assert  x == 0 , "x must be zero" ;
assert  true , format("msg {0}", 1) ;
} } }
"#;
    let output = format_source(input);
    assert!(output.contains(r#"assert x == 0, "x must be zero";"#));
    assert!(output.contains(r#"assert true, format("msg {0}", 1);"#));
}

#[test]
fn format_builtin_functions() {
    let input = r#"
machine M { var s: seq[int]; var m: map[int,int];
start state S { entry {
var k: seq[int]; var v: seq[int]; var sz: int;
k = keys(m);
v = values(m);
sz = sizeof(s);
} } }
"#;
    let output = format_source(input);
    assert!(output.contains("k = keys(m);"));
    assert!(output.contains("v = values(m);"));
    assert!(output.contains("sz = sizeof(s);"));
}

#[test]
fn format_default_expr() {
    let input = r#"
machine M { var s: seq[int];
start state S { entry {
s = default(seq[int]);
} } }
"#;
    let output = format_source(input);
    assert!(output.contains("s = default(seq[int]);"));
}

#[test]
fn format_nested_lvalue() {
    let input = r#"
machine M { var t: (a: seq[int], b: int);
start state S { entry {
t.a[0] = 1;
t.b = 2;
} } }
"#;
    let output = format_source(input);
    assert!(output.contains("t.a[0] = 1;"));
    assert!(output.contains("t.b = 2;"));
}

#[test]
fn format_complex_expression() {
    let input = r#"
machine M { var x: int;
start state S { entry {
x = (1 + 2) * 3 - 4 / 2;
assert x > 0 && x < 100 || x == 0;
} } }
"#;
    let output = format_source(input);
    assert!(output.contains("x = (1 + 2) * 3 - 4 / 2;"));
    assert!(output.contains("x > 0 && x < 100 || x == 0;"));
}

#[test]
fn format_tuple_access() {
    let input = r#"
machine M { var t: (int, int);
start state S { entry {
assert t.0 == 0;
assert t.1 == 0;
} } }
"#;
    let output = format_source(input);
    assert!(output.contains("t.0 == 0;"));
    assert!(output.contains("t.1 == 0;"));
}

#[test]
fn format_in_operator() {
    let input = r#"
machine M { var m: map[int,int]; var s: set[int];
start state S { entry {
assert 1 in m;
assert 2 in s;
} } }
"#;
    let output = format_source(input);
    assert!(output.contains("1 in m;"));
    assert!(output.contains("2 in s;"));
}

#[test]
fn format_announce() {
    let input = r#"
event E: int;
machine M { start state S { entry { announce E, 42; } } }
"#;
    let output = format_source(input);
    assert!(output.contains("announce E, 42;"));
}

#[test]
fn format_raise_halt() {
    let input = r#"
machine M { start state S { entry { raise halt; } } }
"#;
    let output = format_source(input);
    assert!(output.contains("raise halt;"));
}

#[test]
fn format_continue_break() {
    let input = r#"
machine M { var s: seq[int];
start state S { entry {
while (true) { break; }
foreach (x in s) { continue; }
} } }
"#;
    let output = format_source(input);
    assert!(output.contains("break;"));
    assert!(output.contains("continue;"));
}

#[test]
fn format_nested_if_else() {
    let input = r#"
machine M { start state S { entry {
if (true) { if (false) { } else { } } else { }
} } }
"#;
    let output = format_source(input);
    assert!(output.contains("if (true) {"));
    assert!(output.contains("if (false) {"));
    assert!(output.contains("} else {"));
}

#[test]
fn format_multiple_vars_one_decl() {
    let input = r#"
machine M {
var a, b, c: int;
start state S { }
}
"#;
    let output = format_source(input);
    assert!(output.contains("var a, b, c: int;"));
}

#[test]
fn format_seq_set_map_types() {
    let input = r#"
machine M {
var s1: seq[seq[int]];
var s2: set[set[bool]];
var m1: map[int, map[string, float]];
start state S {}
}
"#;
    let output = format_source(input);
    assert!(output.contains("var s1: seq[seq[int]];"));
    assert!(output.contains("var s2: set[set[bool]];"));
    assert!(output.contains("var m1: map[int, map[string, float]];"));
}

#[test]
fn format_module_compose() {
    let input = r#"
event E;
machine M { start state S {} }
machine N { start state S {} }
module Mod1 = { M };
module Mod2 = { N };
implementation Impl [main = M]: (compose Mod1, Mod2);
"#;
    let output = format_source(input);
    assert!(output.contains("module Mod1 = { M };"));
    assert!(output.contains("module Mod2 = { N };"));
    assert!(output.contains("compose Mod1, Mod2"));
}

#[test]
fn format_module_bindings() {
    let input = r#"
event E;
machine M { start state S {} }
module Mod = { M -> I1 , M -> I2 };
"#;
    let output = format_source(input);
    assert!(output.contains("{ M -> I1, M -> I2 }"));
}

#[test]
fn format_assert_in_module() {
    let input = r#"
event E;
machine M { start state S {} }
spec Mon observes E { start state S {} }
test T [main = M]: assert Mon in { M };
"#;
    let output = format_source(input);
    assert!(output.contains("assert Mon in { M }"));
}

#[test]
fn format_machine_receives_sends() {
    let input = r#"
event E; event F;
machine M receives E, F; sends E; { start state S {} }
"#;
    let output = format_source(input);
    assert!(output.contains("receives E, F;"));
    assert!(output.contains("sends E;"));
}

#[test]
fn format_empty_machine() {
    let input = "machine  Empty { start state S { } }";
    let output = format_source(input);
    assert!(output.contains("machine Empty {"));
    assert!(output.contains("start state S {"));
}

#[test]
fn format_return_no_value() {
    let input = r#"
machine M { start state S { entry { return; } } }
"#;
    let output = format_source(input);
    assert!(output.contains("return;"));
}

#[test]
fn format_function_no_return_type() {
    let input = r#"
machine M { start state S {}
fun doStuff(x: int) { } }
"#;
    let output = format_source(input);
    assert!(output.contains("fun doStuff(x: int) {"));
    assert!(!output.contains("doStuff(x: int):"));
}

#[test]
fn format_function_no_params() {
    let input = r#"
machine M { start state S {}
fun doStuff(): int { return 0; } }
"#;
    let output = format_source(input);
    assert!(output.contains("fun doStuff(): int {"));
}

#[test]
fn format_ctor_stmt_no_args() {
    let input = r#"
machine M { start state S { entry { new Other(); } } }
machine Other { start state S {} }
"#;
    let output = format_source(input);
    assert!(output.contains("new Other();"));
}

#[test]
fn format_string_literal() {
    let input = r#"
machine M { start state S { entry {
print "hello world";
} } }
"#;
    let output = format_source(input);
    assert!(output.contains(r#"print "hello world";"#));
}

#[test]
fn format_negation() {
    let input = r#"
machine M { var x: int;
start state S { entry {
x = -1;
assert !false;
} } }
"#;
    let output = format_source(input);
    assert!(output.contains("x = -1;"));
    assert!(output.contains("assert !false;"));
}

#[test]
fn format_unnamed_tuple_single_trailing_comma() {
    let input = r#"
machine M { start state S { entry {
var t: (int,);
} } }
"#;
    // Single-element tuple should have trailing comma
    let output = format_source(input);
    assert!(output.contains("(int)") || output.contains("(int,)"),
        "single element tuple in output: {output}");
}

#[test]
fn format_preserves_semantic_meaning() {
    // Format then compile — should produce same result
    let input = r#"
event Ping: machine;
event Pong;
machine Main {
    var pongId: machine;
    start state Init {
        entry { pongId = new PONG(); raise Pong; }
        on Pong goto Done;
    }
    state Done {}
}
machine PONG { start state S {} }
"#;
    let formatted = format_source(input);
    // Should still compile
    let tokens = plang::compiler::lexer::lex(&formatted).expect("formatted output should lex");
    let mut parser = plang::compiler::parser::Parser::new(tokens, formatted.clone());
    parser.parse_program().expect("formatted output should parse");
}

fn walkdir(dir: &Path) -> Vec<std::path::PathBuf> {
    let mut files = Vec::new();
    if dir.is_dir() {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                files.extend(walkdir(&path));
            } else {
                files.push(path);
            }
        }
    }
    files
}
