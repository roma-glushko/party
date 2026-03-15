use std::path::Path;

fn run_test(test_dir: &str, expected: Expected) {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("testdata")
        .join(test_dir);

    assert!(dir.exists(), "Test directory not found: {}", dir.display());

    match expected {
        Expected::Correct => {
            let program = party::compiler::compile(&dir)
                .expect("Expected successful compilation but got errors");
            party::checker::check(&program)
                .expect("Expected model checking to pass but got violation");
        }
        Expected::StaticError => {
            let result = party::compiler::compile(&dir);
            assert!(result.is_err(), "Expected compilation error but program compiled successfully");
        }
        Expected::DynamicError => {
            let program = party::compiler::compile(&dir)
                .expect("Expected successful compilation (with runtime error) but got compile errors");
            let result = party::checker::check(&program);
            assert!(result.is_err(), "Expected model checking violation but checking passed");
        }
    }
}

#[derive(Clone, Copy)]
enum Expected { Correct, StaticError, DynamicError }
use Expected::*;

// =============================================================================
// Feature1SMLevelDecls — State Machine Declarations
// =============================================================================

mod feature1_correct {
    use super::*;
    #[test] fn bug_repro() { run_test("Feature1SMLevelDecls/Correct/BugRepro", Correct); }
    #[test] fn bug_repro1() { run_test("Feature1SMLevelDecls/Correct/BugRepro1", Correct); }
    #[test] fn bug_repro_many_events() { run_test("Feature1SMLevelDecls/Correct/BugReproManyEvents", Correct); }
    #[test] fn entry_named_function() { run_test("Feature1SMLevelDecls/Correct/EntryNamedFunction", Correct); }
    #[test] fn entry_named_function1() { run_test("Feature1SMLevelDecls/Correct/EntryNamedFunction1", Correct); }
    #[test] fn monitors() { run_test("Feature1SMLevelDecls/Correct/Monitors", Correct); }
    #[test] fn more_than_32_events() { run_test("Feature1SMLevelDecls/Correct/MoreThan32Events", Correct); }
    #[test] fn param_test() { run_test("Feature1SMLevelDecls/Correct/ParamTest", Correct); }
    #[test] fn ping_pong() { run_test("Feature1SMLevelDecls/Correct/PingPong", Correct); }
    #[test] fn static_functions() { run_test("Feature1SMLevelDecls/Correct/StaticFunctions", Correct); }
    #[test] fn transition_function() { run_test("Feature1SMLevelDecls/Correct/TransitionFunction", Correct); }
    #[test] fn bug1() { run_test("Feature1SMLevelDecls/Correct/bug1", Correct); }
    #[test] fn bug3() { run_test("Feature1SMLevelDecls/Correct/bug3", Correct); }
    #[test] fn bug4() { run_test("Feature1SMLevelDecls/Correct/bug4", Correct); }
    #[test] fn function_any() { run_test("Feature1SMLevelDecls/Correct/functionAny", Correct); }
    #[test] fn monitor_observes() { run_test("Feature1SMLevelDecls/Correct/monitorobserves", Correct); }
    #[test] fn recursive_function_calls() { run_test("Feature1SMLevelDecls/Correct/recursivefunctioncalls", Correct); }
}

mod feature1_static_error {
    use super::*;
    #[test] fn action_and_transition_same_event() { run_test("Feature1SMLevelDecls/StaticError/ActionAndTransitionSameEvent", StaticError); }
    #[test] fn anon_funs() { run_test("Feature1SMLevelDecls/StaticError/AnonFuns", StaticError); }
    #[test] fn assign_bad_lhs() { run_test("Feature1SMLevelDecls/StaticError/AssignBadLhs", StaticError); }
    #[test] fn create_spec_machine() { run_test("Feature1SMLevelDecls/StaticError/CreateSpecMachine", StaticError); }
    #[test] fn defer_ignore_same_event() { run_test("Feature1SMLevelDecls/StaticError/DeferIgnoreSameEvent", StaticError); }
    #[test] fn deferred_null_event() { run_test("Feature1SMLevelDecls/StaticError/DeferredNullEvent", StaticError); }
    #[test] fn entry_named_function1() { run_test("Feature1SMLevelDecls/StaticError/EntryNamedFunction1", StaticError); }
    #[test] fn entry_named_function2() { run_test("Feature1SMLevelDecls/StaticError/EntryNamedFunction2", StaticError); }
    #[test] fn entry_named_function3() { run_test("Feature1SMLevelDecls/StaticError/EntryNamedFunction3", StaticError); }
    #[test] fn entry_returns_value() { run_test("Feature1SMLevelDecls/StaticError/EntryReturnsValue", StaticError); }
    #[test] fn event_deferred_do_same_state() { run_test("Feature1SMLevelDecls/StaticError/EventDeferredDoSameState", StaticError); }
    #[test] fn event_deferred_handled_same_state() { run_test("Feature1SMLevelDecls/StaticError/EventDeferredHandledSameState", StaticError); }
    #[test] fn event_deferred_trans_do_same_state() { run_test("Feature1SMLevelDecls/StaticError/EventDeferredTransDoSameState", StaticError); }
    #[test] fn function_missing_args() { run_test("Feature1SMLevelDecls/StaticError/FunctionMissingArgs", StaticError); }
    #[test] fn function_missing_return() { run_test("Feature1SMLevelDecls/StaticError/FunctionMissingReturn", StaticError); }
    #[test] fn function_returns_nothing_in_assignment() { run_test("Feature1SMLevelDecls/StaticError/FunctionReturnsNothingInAssignment", StaticError); }
    #[test] fn function_returns_wrong_type() { run_test("Feature1SMLevelDecls/StaticError/FunctionReturnsWrongType", StaticError); }
    #[test] fn has_entry_args() { run_test("Feature1SMLevelDecls/StaticError/HasEntryArgs", StaticError); }
    #[test] fn has_exit_args() { run_test("Feature1SMLevelDecls/StaticError/HasExitArgs", StaticError); }
    #[test] fn ignored_null_event() { run_test("Feature1SMLevelDecls/StaticError/IgnoredNullEvent", StaticError); }
    #[test] fn lexer_error1() { run_test("Feature1SMLevelDecls/StaticError/LexerError1", StaticError); }
    #[test] fn lexer_error2() { run_test("Feature1SMLevelDecls/StaticError/LexerError2", StaticError); }
    #[test] fn machine_no_start_state() { run_test("Feature1SMLevelDecls/StaticError/MachineNoStartState", StaticError); }
    #[test] fn no_defer_in_spec_machine() { run_test("Feature1SMLevelDecls/StaticError/NoDeferInSpecMachine", StaticError); }
    #[test] fn no_send_in_spec_machine() { run_test("Feature1SMLevelDecls/StaticError/NoSendInSpecMachine", StaticError); }
    #[test] fn non_existent_do_fun() { run_test("Feature1SMLevelDecls/StaticError/NonExistentDoFun", StaticError); }
    #[test] fn non_existent_entry_fun() { run_test("Feature1SMLevelDecls/StaticError/NonExistentEntryFun", StaticError); }
    #[test] fn non_existent_exit_fun() { run_test("Feature1SMLevelDecls/StaticError/NonExistentExitFun", StaticError); }
    #[test] fn non_existent_goto_fun() { run_test("Feature1SMLevelDecls/StaticError/NonExistentGotoFun", StaticError); }
    #[test] fn null_event_decl() { run_test("Feature1SMLevelDecls/StaticError/NullEventDecl", StaticError); }
    #[test] fn param_test3() { run_test("Feature1SMLevelDecls/StaticError/ParamTest3", StaticError); }
    #[test] fn param_test4() { run_test("Feature1SMLevelDecls/StaticError/ParamTest4", StaticError); }
    #[test] fn partial() { run_test("Feature1SMLevelDecls/StaticError/Partial", StaticError); }
    #[test] fn raise_send_bad_value_spec_machine() { run_test("Feature1SMLevelDecls/StaticError/RaiseSendBadValueSpecMachine", StaticError); }
    #[test] fn raise_send_null_spec_machine() { run_test("Feature1SMLevelDecls/StaticError/RaiseSendNullSpecMachine", StaticError); }
    #[test] fn raised_null_event() { run_test("Feature1SMLevelDecls/StaticError/RaisedNullEvent", StaticError); }
    #[test] fn send_in_monitor() { run_test("Feature1SMLevelDecls/StaticError/SendInMonitor", StaticError); }
    #[test] fn sent_null_event() { run_test("Feature1SMLevelDecls/StaticError/SentNullEvent", StaticError); }
    #[test] fn side_effects_in_monitor() { run_test("Feature1SMLevelDecls/StaticError/SideEffectsInMonitor", StaticError); }
    #[test] fn start_machine_null_param() { run_test("Feature1SMLevelDecls/StaticError/StartMachineNullParam", StaticError); }
    #[test] fn static_function_in_monitor() { run_test("Feature1SMLevelDecls/StaticError/StaticFunctionInMonitor", StaticError); }
    #[test] fn transition_on_null_in_spec_machine() { run_test("Feature1SMLevelDecls/StaticError/TransitionOnNullInSpecMachine", StaticError); }
    #[test] fn undefined_state_in_transition() { run_test("Feature1SMLevelDecls/StaticError/UndefinedStateInTransition", StaticError); }
    #[test] fn function_any_anon() { run_test("Feature1SMLevelDecls/StaticError/functionAnyAnon", StaticError); }
}

mod feature1_dynamic_error {
    use super::*;
    #[test] fn alon_bug() { run_test("Feature1SMLevelDecls/DynamicError/AlonBug", DynamicError); }
    #[test] fn entry_named_function() { run_test("Feature1SMLevelDecls/DynamicError/EntryNamedFunction", DynamicError); }
    #[test] fn max_instances() { run_test("Feature1SMLevelDecls/DynamicError/MaxInstances", DynamicError); }
    #[test] fn param_test2() { run_test("Feature1SMLevelDecls/DynamicError/ParamTest2", DynamicError); }
    #[test] fn static_function_in_monitor() { run_test("Feature1SMLevelDecls/DynamicError/StaticFunctionInMonitor", DynamicError); }
    #[test] fn static_functions1() { run_test("Feature1SMLevelDecls/DynamicError/StaticFunctions1", DynamicError); }
    #[test] fn static_functions2() { run_test("Feature1SMLevelDecls/DynamicError/StaticFunctions2", DynamicError); }
    #[test] fn bug2() { run_test("Feature1SMLevelDecls/DynamicError/bug2", DynamicError); }
}

// =============================================================================
// Feature2Stmts — Statements
// =============================================================================

mod feature2_correct {
    use super::*;
    #[test] fn nondet_fun() { run_test("Feature2Stmts/Correct/NondetFun", Correct); }
    #[test] fn sem_one_machine_34() { run_test("Feature2Stmts/Correct/SEM_ONeMachine_34", Correct); }
    #[test] fn sem_one_machine_33() { run_test("Feature2Stmts/Correct/SEM_OneMachine_33", Correct); }
    #[test] fn sem_one_machine_35() { run_test("Feature2Stmts/Correct/SEM_OneMachine_35", Correct); }
    #[test] fn assert_example() { run_test("Feature2Stmts/Correct/assertExample", Correct); }
    #[test] fn foreach() { run_test("Feature2Stmts/Correct/foreach", Correct); }
    #[test] fn foreach1() { run_test("Feature2Stmts/Correct/foreach1", Correct); }
    #[test] fn foreach2() { run_test("Feature2Stmts/Correct/foreach2", Correct); }
    #[test] fn foreach3() { run_test("Feature2Stmts/Correct/foreach3", Correct); }
    #[test] fn foreach4() { run_test("Feature2Stmts/Correct/foreach4", Correct); }
    #[test] fn goto1() { run_test("Feature2Stmts/Correct/goto1", Correct); }
    #[test] fn goto2() { run_test("Feature2Stmts/Correct/goto2", Correct); }
    #[test] fn goto3() { run_test("Feature2Stmts/Correct/goto3", Correct); }
    #[test] fn goto4() { run_test("Feature2Stmts/Correct/goto4", Correct); }
    #[test] fn linear2() { run_test("Feature2Stmts/Correct/linear2", Correct); }
    #[test] fn new_machine1() { run_test("Feature2Stmts/Correct/newMachine1", Correct); }
    #[test] fn ping_pong_receive4() { run_test("Feature2Stmts/Correct/pingPongReceive4", Correct); }
    #[test] fn ping_pong_receive5() { run_test("Feature2Stmts/Correct/pingPongReceive5", Correct); }
    #[test] fn raise1() { run_test("Feature2Stmts/Correct/raise1", Correct); }
    #[test] fn raise2() { run_test("Feature2Stmts/Correct/raise2", Correct); }
    #[test] fn raise3() { run_test("Feature2Stmts/Correct/raise3", Correct); }
    #[test] fn receive1() { run_test("Feature2Stmts/Correct/receive1", Correct); }
    #[test] fn receive11() { run_test("Feature2Stmts/Correct/receive11", Correct); }
    #[test] fn receive111() { run_test("Feature2Stmts/Correct/receive111", Correct); }
    #[test] fn receive13() { run_test("Feature2Stmts/Correct/receive13", Correct); }
    #[test] fn receive14() { run_test("Feature2Stmts/Correct/receive14", Correct); }
    #[test] fn receive15() { run_test("Feature2Stmts/Correct/receive15", Correct); }
    #[test] fn receive16() { run_test("Feature2Stmts/Correct/receive16", Correct); }
    #[test] fn receive18() { run_test("Feature2Stmts/Correct/receive18", Correct); }
    #[test] fn receive19() { run_test("Feature2Stmts/Correct/receive19", Correct); }
}

mod feature2_static_error {
    use super::*;
    #[test] fn linear_bug_repro1() { run_test("Feature2Stmts/StaticError/LinearBugRepro1", StaticError); }
    #[test] fn transaction_type() { run_test("Feature2Stmts/StaticError/TransactionType", StaticError); }
    #[test] fn break_outside_loop() { run_test("Feature2Stmts/StaticError/breakOutsideLoop", StaticError); }
    #[test] fn continue_outside_loop() { run_test("Feature2Stmts/StaticError/continueOutsideLoop", StaticError); }
    #[test] fn entry_exit() { run_test("Feature2Stmts/StaticError/entryExit", StaticError); }
    #[test] fn event_expr_send_raise() { run_test("Feature2Stmts/StaticError/eventExprSendRaise", StaticError); }
    #[test] fn foreach1() { run_test("Feature2Stmts/StaticError/foreach1", StaticError); }
    #[test] fn foreach2() { run_test("Feature2Stmts/StaticError/foreach2", StaticError); }
    #[test] fn foreach3() { run_test("Feature2Stmts/StaticError/foreach3", StaticError); }
    #[test] fn goto1() { run_test("Feature2Stmts/StaticError/goto1", StaticError); }
    #[test] fn goto2() { run_test("Feature2Stmts/StaticError/goto2", StaticError); }
    #[test] fn goto3() { run_test("Feature2Stmts/StaticError/goto3", StaticError); }
    #[test] fn linear() { run_test("Feature2Stmts/StaticError/linear", StaticError); }
    #[test] fn linear3() { run_test("Feature2Stmts/StaticError/linear3", StaticError); }
    #[test] fn linear4() { run_test("Feature2Stmts/StaticError/linear4", StaticError); }
    #[test] fn lvalues() { run_test("Feature2Stmts/StaticError/lvalues", StaticError); }
    #[test] fn new_machine1() { run_test("Feature2Stmts/StaticError/newMachine1", StaticError); }
    #[test] fn new_machine2() { run_test("Feature2Stmts/StaticError/newMachine2", StaticError); }
    #[test] fn new_machine3() { run_test("Feature2Stmts/StaticError/newMachine3", StaticError); }
    #[test] fn nmd_type() { run_test("Feature2Stmts/StaticError/nmdType", StaticError); }
    #[test] fn raise1() { run_test("Feature2Stmts/StaticError/raise1", StaticError); }
    #[test] fn raise2() { run_test("Feature2Stmts/StaticError/raise2", StaticError); }
    #[test] fn sends() { run_test("Feature2Stmts/StaticError/sends", StaticError); }
    #[test] fn static_fun_return_type() { run_test("Feature2Stmts/StaticError/staticFunReturnType", StaticError); }
}

mod feature2_dynamic_error {
    use super::*;
    #[test] fn goto_stmt1() { run_test("Feature2Stmts/DynamicError/GotoStmt1", DynamicError); }
    #[test] fn goto_stmt2() { run_test("Feature2Stmts/DynamicError/GotoStmt2", DynamicError); }
    #[test] fn nondet_fun() { run_test("Feature2Stmts/DynamicError/NondetFun", DynamicError); }
    #[test] fn break1() { run_test("Feature2Stmts/DynamicError/break1", DynamicError); }
    #[test] fn continue1() { run_test("Feature2Stmts/DynamicError/continue1", DynamicError); }
    #[test] fn foreach() { run_test("Feature2Stmts/DynamicError/foreach", DynamicError); }
    #[test] fn foreach2() { run_test("Feature2Stmts/DynamicError/foreach2", DynamicError); }
    #[test] fn foreach3() { run_test("Feature2Stmts/DynamicError/foreach3", DynamicError); }
    #[test] fn foreach4() { run_test("Feature2Stmts/DynamicError/foreach4", DynamicError); }
    #[test] fn goto2() { run_test("Feature2Stmts/DynamicError/goto2", DynamicError); }
    #[test] fn goto3() { run_test("Feature2Stmts/DynamicError/goto3", DynamicError); }
    #[test] fn goto4() { run_test("Feature2Stmts/DynamicError/goto4", DynamicError); }
    #[test] fn new_machine1() { run_test("Feature2Stmts/DynamicError/newMachine1", DynamicError); }
    #[test] fn ping_pong_receive4() { run_test("Feature2Stmts/DynamicError/pingPongReceive4", DynamicError); }
    #[test] fn ping_pong_receive5() { run_test("Feature2Stmts/DynamicError/pingPongReceive5", DynamicError); }
    #[test] fn raise1() { run_test("Feature2Stmts/DynamicError/raise1", DynamicError); }
    #[test] fn raise2() { run_test("Feature2Stmts/DynamicError/raise2", DynamicError); }
    #[test] fn raise3() { run_test("Feature2Stmts/DynamicError/raise3", DynamicError); }
    #[test] fn receive2() { run_test("Feature2Stmts/DynamicError/receive2", DynamicError); }
    #[test] fn receive3() { run_test("Feature2Stmts/DynamicError/receive3", DynamicError); }
    #[test] fn receive4() { run_test("Feature2Stmts/DynamicError/receive4", DynamicError); }
    #[test] fn receive6() { run_test("Feature2Stmts/DynamicError/receive6", DynamicError); }
    #[test] fn receive7() { run_test("Feature2Stmts/DynamicError/receive7", DynamicError); }
    #[test] fn receive8() { run_test("Feature2Stmts/DynamicError/receive8", DynamicError); }
    #[test] fn receive9() { run_test("Feature2Stmts/DynamicError/receive9", DynamicError); }
    #[test] fn receive10() { run_test("Feature2Stmts/DynamicError/receive10", DynamicError); }
    #[test] fn receive11() { run_test("Feature2Stmts/DynamicError/receive11", DynamicError); }
    #[test] fn receive11_1() { run_test("Feature2Stmts/DynamicError/receive11_1", DynamicError); }
    #[test] fn receive12() { run_test("Feature2Stmts/DynamicError/receive12", DynamicError); }
    #[test] fn receive17() { run_test("Feature2Stmts/DynamicError/receive17", DynamicError); }
}

// =============================================================================
// Feature3Exprs — Expressions
// =============================================================================

mod feature3_correct {
    use super::*;
    #[test] fn expr_operators_asserts() { run_test("Feature3Exprs/Correct/ExprOperatorsAsserts", Correct); }
    #[test] fn memory_leak_repro() { run_test("Feature3Exprs/Correct/MemoryLeakRepro", Correct); }
    #[test] fn mod_expr1() { run_test("Feature3Exprs/Correct/ModExpr1", Correct); }
    #[test] fn nested_fun() { run_test("Feature3Exprs/Correct/NestedFun", Correct); }
    #[test] fn non_det_function_in_expr() { run_test("Feature3Exprs/Correct/NonDetFunctionInExpr", Correct); }
    #[test] fn short_circuit_eval() { run_test("Feature3Exprs/Correct/ShortCircuitEval", Correct); }
    #[test] fn single_named_tuple1() { run_test("Feature3Exprs/Correct/SingleNamedTuple1", Correct); }
    #[test] fn single_named_tuple2() { run_test("Feature3Exprs/Correct/SingleNamedTuple2", Correct); }
    #[test] fn assert_message() { run_test("Feature3Exprs/Correct/assertMessage", Correct); }
    #[test] fn assert_message2() { run_test("Feature3Exprs/Correct/assertMessage2", Correct); }
    #[test] fn cast1() { run_test("Feature3Exprs/Correct/cast1", Correct); }
    #[test] fn cast2() { run_test("Feature3Exprs/Correct/cast2", Correct); }
    #[test] fn cast3() { run_test("Feature3Exprs/Correct/cast3", Correct); }
    #[test] fn choose_expr1() { run_test("Feature3Exprs/Correct/chooseExpr1", Correct); }
    #[test] fn choose_expr2() { run_test("Feature3Exprs/Correct/chooseExpr2", Correct); }
    #[test] fn events1() { run_test("Feature3Exprs/Correct/events1", Correct); }
    #[test] fn float1() { run_test("Feature3Exprs/Correct/float1", Correct); }
    #[test] fn float2() { run_test("Feature3Exprs/Correct/float2", Correct); }
    #[test] fn float3() { run_test("Feature3Exprs/Correct/float3", Correct); }
    #[test] fn issue511() { run_test("Feature3Exprs/Correct/issue511", Correct); }
}

mod feature3_static_error {
    use super::*;
    #[test] fn exprs_operators() { run_test("Feature3Exprs/StaticError/ExprsOperators", StaticError); }
    #[test] fn in_operator() { run_test("Feature3Exprs/StaticError/InOperator", StaticError); }
    #[test] fn machine_field_access() { run_test("Feature3Exprs/StaticError/MachineFieldAccess", StaticError); }
    #[test] fn machine_field_access_in_expr() { run_test("Feature3Exprs/StaticError/MachineFieldAccessInExpr", StaticError); }
    #[test] fn machine_field_access_nested() { run_test("Feature3Exprs/StaticError/MachineFieldAccessNested", StaticError); }
    #[test] fn mod_expr1() { run_test("Feature3Exprs/StaticError/ModExpr1", StaticError); }
    #[test] fn choose3() { run_test("Feature3Exprs/StaticError/choose3", StaticError); }
    #[test] fn fields() { run_test("Feature3Exprs/StaticError/fields", StaticError); }
    #[test] fn too_many_choices() { run_test("Feature3Exprs/StaticError/tooManyChoices", StaticError); }
}

mod feature3_dynamic_error {
    use super::*;
    #[test] fn func_in_expr() { run_test("Feature3Exprs/DynamicError/FuncInExpr", DynamicError); }
    #[test] fn in_operator() { run_test("Feature3Exprs/DynamicError/InOperator", DynamicError); }
    #[test] fn mod_expr1() { run_test("Feature3Exprs/DynamicError/ModExpr1", DynamicError); }
    #[test] fn choose_expr2() { run_test("Feature3Exprs/DynamicError/chooseExpr2", DynamicError); }
    #[test] fn too_many_choices_int() { run_test("Feature3Exprs/DynamicError/tooManyChoicesInt", DynamicError); }
    #[test] fn too_many_choices_map() { run_test("Feature3Exprs/DynamicError/tooManyChoicesMap", DynamicError); }
    #[test] fn too_many_choices_seq() { run_test("Feature3Exprs/DynamicError/tooManyChoicesSeq", DynamicError); }
    #[test] fn too_many_choices_set() { run_test("Feature3Exprs/DynamicError/tooManyChoicesSet", DynamicError); }
}

// =============================================================================
// Feature4DataTypes — Data Types
// =============================================================================

mod feature4_correct {
    use super::*;
    #[test] fn cast_in_exprs_asserts() { run_test("Feature4DataTypes/Correct/CastInExprsAsserts", Correct); }
    #[test] fn enum_type() { run_test("Feature4DataTypes/Correct/EnumType", Correct); }
    #[test] fn foreign_types() { run_test("Feature4DataTypes/Correct/ForeignTypes", Correct); }
    #[test] fn return_issue() { run_test("Feature4DataTypes/Correct/ReturnIssue", Correct); }
    #[test] fn set_impl0() { run_test("Feature4DataTypes/Correct/SetImpl0", Correct); }
    #[test] fn set_impl2() { run_test("Feature4DataTypes/Correct/SetImpl2", Correct); }
    #[test] fn set_impl3() { run_test("Feature4DataTypes/Correct/SetImpl3", Correct); }
    #[test] fn any_type_null_value() { run_test("Feature4DataTypes/Correct/anyTypeNullValue", Correct); }
    #[test] fn enum1() { run_test("Feature4DataTypes/Correct/enum1", Correct); }
    #[test] fn enum2() { run_test("Feature4DataTypes/Correct/enum2", Correct); }
    #[test] fn enum3() { run_test("Feature4DataTypes/Correct/enum3", Correct); }
    #[test] fn enum4() { run_test("Feature4DataTypes/Correct/enum4", Correct); }
    #[test] fn float1() { run_test("Feature4DataTypes/Correct/float1", Correct); }
    #[test] fn float4() { run_test("Feature4DataTypes/Correct/float4", Correct); }
    #[test] fn nested_typedef() { run_test("Feature4DataTypes/Correct/nestedTypedef", Correct); }
    #[test] fn non_atomic_data_types() { run_test("Feature4DataTypes/Correct/nonAtomicDataTypes", Correct); }
    #[test] fn non_atomic_data_types12() { run_test("Feature4DataTypes/Correct/nonAtomicDataTypes12", Correct); }
    #[test] fn non_atomic_data_types13() { run_test("Feature4DataTypes/Correct/nonAtomicDataTypes13", Correct); }
    #[test] fn non_atomic_data_types16() { run_test("Feature4DataTypes/Correct/nonAtomicDataTypes16", Correct); }
    #[test] fn non_atomic_data_types_all_asserts() { run_test("Feature4DataTypes/Correct/nonAtomicDataTypesAllAsserts", Correct); }
    #[test] fn seq1() { run_test("Feature4DataTypes/Correct/seq1", Correct); }
    #[test] fn string0() { run_test("Feature4DataTypes/Correct/string0", Correct); }
    #[test] fn stringcomp() { run_test("Feature4DataTypes/Correct/stringcomp", Correct); }
    #[test] fn typedef() { run_test("Feature4DataTypes/Correct/typedef", Correct); }
    #[test] fn typedef2() { run_test("Feature4DataTypes/Correct/typedef2", Correct); }
    #[test] fn typedef3() { run_test("Feature4DataTypes/Correct/typedef3", Correct); }
}

mod feature4_static_error {
    use super::*;
    #[test] fn cast_in_exprs() { run_test("Feature4DataTypes/StaticError/CastInExprs", StaticError); }
    #[test] fn enum_type() { run_test("Feature4DataTypes/StaticError/EnumType", StaticError); }
    #[test] fn event_sets_1() { run_test("Feature4DataTypes/StaticError/EventSets_1", StaticError); }
    #[test] fn event_sets_2() { run_test("Feature4DataTypes/StaticError/EventSets_2", StaticError); }
    #[test] fn event_sets_3() { run_test("Feature4DataTypes/StaticError/EventSets_3", StaticError); }
    #[test] fn event_sets_4() { run_test("Feature4DataTypes/StaticError/EventSets_4", StaticError); }
    #[test] fn event_sets_5() { run_test("Feature4DataTypes/StaticError/EventSets_5", StaticError); }
    #[test] fn machine_names_1() { run_test("Feature4DataTypes/StaticError/MachineNames_1", StaticError); }
    #[test] fn set_access() { run_test("Feature4DataTypes/StaticError/SetAccess", StaticError); }
    #[test] fn set_impl2() { run_test("Feature4DataTypes/StaticError/SetImpl2", StaticError); }
    #[test] fn enum1() { run_test("Feature4DataTypes/StaticError/enum1", StaticError); }
    #[test] fn float4() { run_test("Feature4DataTypes/StaticError/float4", StaticError); }
    #[test] fn function_typos() { run_test("Feature4DataTypes/StaticError/function_Typos", StaticError); }
    #[test] fn named_duplicate_field() { run_test("Feature4DataTypes/StaticError/namedDuplicateField", StaticError); }
    #[test] fn named_duplicate_field2() { run_test("Feature4DataTypes/StaticError/namedDuplicateField2", StaticError); }
    #[test] fn named_tuple1() { run_test("Feature4DataTypes/StaticError/namedTuple1", StaticError); }
    #[test] fn named_tuple2() { run_test("Feature4DataTypes/StaticError/namedTuple2", StaticError); }
    #[test] fn non_atomic_data_types() { run_test("Feature4DataTypes/StaticError/nonAtomicDataTypes", StaticError); }
    #[test] fn null_comparison() { run_test("Feature4DataTypes/StaticError/nullComparison", StaticError); }
    #[test] fn null_type_assign() { run_test("Feature4DataTypes/StaticError/nullTypeAssign", StaticError); }
    #[test] fn payload_actions() { run_test("Feature4DataTypes/StaticError/payloadActions", StaticError); }
    #[test] fn payload_actions_funs() { run_test("Feature4DataTypes/StaticError/payloadActionsFuns", StaticError); }
    #[test] fn payload_entry() { run_test("Feature4DataTypes/StaticError/payloadEntry", StaticError); }
    #[test] fn payload_entry_1() { run_test("Feature4DataTypes/StaticError/payloadEntry_1", StaticError); }
    #[test] fn payload_transitions() { run_test("Feature4DataTypes/StaticError/payloadTransitions", StaticError); }
    #[test] fn payloads() { run_test("Feature4DataTypes/StaticError/payloads", StaticError); }
    #[test] fn string2() { run_test("Feature4DataTypes/StaticError/string2", StaticError); }
    #[test] fn typedef() { run_test("Feature4DataTypes/StaticError/typedef", StaticError); }
    #[test] fn typedef2() { run_test("Feature4DataTypes/StaticError/typedef2", StaticError); }
}

mod feature4_dynamic_error {
    use super::*;
    #[test] fn cast_in_exprs1() { run_test("Feature4DataTypes/DynamicError/CastInExprs1", DynamicError); }
    #[test] fn cast_in_exprs2() { run_test("Feature4DataTypes/DynamicError/CastInExprs2", DynamicError); }
    #[test] fn cast_in_exprs3() { run_test("Feature4DataTypes/DynamicError/CastInExprs3", DynamicError); }
    #[test] fn cast_in_exprs4() { run_test("Feature4DataTypes/DynamicError/CastInExprs4", DynamicError); }
    #[test] fn cast_in_exprs5() { run_test("Feature4DataTypes/DynamicError/CastInExprs5", DynamicError); }
    #[test] fn cast_in_exprs6() { run_test("Feature4DataTypes/DynamicError/CastInExprs6", DynamicError); }
    #[test] fn enum_type1() { run_test("Feature4DataTypes/DynamicError/EnumType1", DynamicError); }
    #[test] fn foreign_types() { run_test("Feature4DataTypes/DynamicError/ForeignTypes", DynamicError); }
    #[test] fn set_impl1() { run_test("Feature4DataTypes/DynamicError/SetImpl1", DynamicError); }
    #[test] fn any_type() { run_test("Feature4DataTypes/DynamicError/anyType", DynamicError); }
    #[test] fn any_type1() { run_test("Feature4DataTypes/DynamicError/anyType1", DynamicError); }
    #[test] fn any_type2() { run_test("Feature4DataTypes/DynamicError/anyType2", DynamicError); }
    #[test] fn any_type3() { run_test("Feature4DataTypes/DynamicError/anyType3", DynamicError); }
    #[test] fn enum1() { run_test("Feature4DataTypes/DynamicError/enum1", DynamicError); }
    #[test] fn non_atomic_data_types() { run_test("Feature4DataTypes/DynamicError/nonAtomicDataTypes", DynamicError); }
    #[test] fn non_atomic_data_types1() { run_test("Feature4DataTypes/DynamicError/nonAtomicDataTypes1", DynamicError); }
    #[test] fn non_atomic_data_types2() { run_test("Feature4DataTypes/DynamicError/nonAtomicDataTypes2", DynamicError); }
    #[test] fn non_atomic_data_types3() { run_test("Feature4DataTypes/DynamicError/nonAtomicDataTypes3", DynamicError); }
    #[test] fn non_atomic_data_types4() { run_test("Feature4DataTypes/DynamicError/nonAtomicDataTypes4", DynamicError); }
    #[test] fn non_atomic_data_types5() { run_test("Feature4DataTypes/DynamicError/nonAtomicDataTypes5", DynamicError); }
    #[test] fn non_atomic_data_types6() { run_test("Feature4DataTypes/DynamicError/nonAtomicDataTypes6", DynamicError); }
    #[test] fn non_atomic_data_types7() { run_test("Feature4DataTypes/DynamicError/nonAtomicDataTypes7", DynamicError); }
    #[test] fn non_atomic_data_types8() { run_test("Feature4DataTypes/DynamicError/nonAtomicDataTypes8", DynamicError); }
    #[test] fn non_atomic_data_types9() { run_test("Feature4DataTypes/DynamicError/nonAtomicDataTypes9", DynamicError); }
    #[test] fn non_atomic_data_types10() { run_test("Feature4DataTypes/DynamicError/nonAtomicDataTypes10", DynamicError); }
    #[test] fn non_atomic_data_types11() { run_test("Feature4DataTypes/DynamicError/nonAtomicDataTypes11", DynamicError); }
    #[test] fn non_atomic_data_types14() { run_test("Feature4DataTypes/DynamicError/nonAtomicDataTypes14", DynamicError); }
    #[test] fn non_atomic_data_types15() { run_test("Feature4DataTypes/DynamicError/nonAtomicDataTypes15", DynamicError); }
    #[test] fn seq1() { run_test("Feature4DataTypes/DynamicError/seq1", DynamicError); }
    #[test] fn string1() { run_test("Feature4DataTypes/DynamicError/string1", DynamicError); }
    #[test] fn typedef1() { run_test("Feature4DataTypes/DynamicError/typedef1", DynamicError); }
    #[test] fn typedef2() { run_test("Feature4DataTypes/DynamicError/typedef2", DynamicError); }
}

// =============================================================================
// Feature5ModuleSystem — Module System
// =============================================================================

mod feature5_correct {
    use super::*;
    #[test] fn elevator() { run_test("Feature5ModuleSystem/Correct/Elevator", Correct); }
    #[test] fn elevator_mod() { run_test("Feature5ModuleSystem/Correct/ElevatorMod", Correct); }
    #[test] fn interface_test() { run_test("Feature5ModuleSystem/Correct/InterfaceTest", Correct); }
    #[test] fn osr() { run_test("Feature5ModuleSystem/Correct/OSR", Correct); }
    #[test] fn ping_pong_ding_dong_mod() { run_test("Feature5ModuleSystem/Correct/PingPongDingDongMod", Correct); }
}

// =============================================================================
// Combined — Cross-Feature
// =============================================================================

mod combined_correct {
    use super::*;
    #[test] fn null_payload() { run_test("Combined/Correct/nullPayload", Correct); }
    #[test] fn variable_type() { run_test("Combined/Correct/variableType", Correct); }
}

mod combined_static_error {
    use super::*;
    #[test] fn control_impure_enclosed_fun_calls() { run_test("Combined/StaticError/ControlImpureEnclosedFunCalls", StaticError); }
    #[test] fn control_impure_in_exit1() { run_test("Combined/StaticError/ControlImpureInExit1", StaticError); }
    #[test] fn control_impure_in_exit2() { run_test("Combined/StaticError/ControlImpureInExit2", StaticError); }
    #[test] fn control_impure_in_exit3() { run_test("Combined/StaticError/ControlImpureInExit3", StaticError); }
    #[test] fn control_impure_in_exit4() { run_test("Combined/StaticError/ControlImpureInExit4", StaticError); }
    #[test] fn control_impure_in_exit5() { run_test("Combined/StaticError/ControlImpureInExit5", StaticError); }
    #[test] fn control_impure_in_exit6() { run_test("Combined/StaticError/ControlImpureInExit6", StaticError); }
    #[test] fn control_impure_in_goto1() { run_test("Combined/StaticError/ControlImpureInGoto1", StaticError); }
    #[test] fn control_impure_in_goto2() { run_test("Combined/StaticError/ControlImpureInGoto2", StaticError); }
    #[test] fn control_impure_in_goto3() { run_test("Combined/StaticError/ControlImpureInGoto3", StaticError); }
    #[test] fn control_impure_in_goto4() { run_test("Combined/StaticError/ControlImpureInGoto4", StaticError); }
    #[test] fn control_impure_in_goto5() { run_test("Combined/StaticError/ControlImpureInGoto5", StaticError); }
    #[test] fn control_impure_in_goto6() { run_test("Combined/StaticError/ControlImpureInGoto6", StaticError); }
    #[test] fn control_impure_in_goto7() { run_test("Combined/StaticError/ControlImpureInGoto7", StaticError); }
    #[test] fn duplicate_actions() { run_test("Combined/StaticError/DuplicateActions", StaticError); }
    #[test] fn duplicate_start() { run_test("Combined/StaticError/DuplicateStart", StaticError); }
    #[test] fn duplicate_transitions() { run_test("Combined/StaticError/DuplicateTransitions", StaticError); }
    #[test] fn duplicates1() { run_test("Combined/StaticError/Duplicates1", StaticError); }
    #[test] fn duplicates2() { run_test("Combined/StaticError/Duplicates2", StaticError); }
    #[test] fn duplicates3() { run_test("Combined/StaticError/Duplicates3", StaticError); }
    #[test] fn duplicates4() { run_test("Combined/StaticError/Duplicates4", StaticError); }
    #[test] fn duplicates5() { run_test("Combined/StaticError/Duplicates5", StaticError); }
    #[test] fn duplicates6() { run_test("Combined/StaticError/Duplicates6", StaticError); }
    #[test] fn duplicates7() { run_test("Combined/StaticError/Duplicates7", StaticError); }
    #[test] fn duplicates8() { run_test("Combined/StaticError/Duplicates8", StaticError); }
    #[test] fn duplicates9() { run_test("Combined/StaticError/Duplicates9", StaticError); }
    #[test] fn duplicates10() { run_test("Combined/StaticError/Duplicates10", StaticError); }
    #[test] fn function_not_defined() { run_test("Combined/StaticError/FunctionNotDefined", StaticError); }
    #[test] fn pop_in_exit_fun() { run_test("Combined/StaticError/PopInExitFun", StaticError); }
    #[test] fn raise_in_exit_fun() { run_test("Combined/StaticError/RaiseInExitFun", StaticError); }
}

// =============================================================================
// Integration — Multi-Machine Communication
// =============================================================================

mod integration_correct {
    use super::*;
    #[test] fn bounded_async() { run_test("Integration/Correct/BoundedAsync", Correct); }
    #[test] fn german() { run_test("Integration/Correct/German", Correct); }
    #[test] fn ping_pong() { run_test("Integration/Correct/PingPong", Correct); }
    #[test] fn ping_pong_defer() { run_test("Integration/Correct/PingPongDefer", Correct); }
    #[test] fn ping_pong_ding_dong() { run_test("Integration/Correct/PingPongDingDong", Correct); }
    #[test] fn ping_pong_monitor() { run_test("Integration/Correct/PingPongMonitor", Correct); }
    #[test] fn ping_pong_non_det() { run_test("Integration/Correct/PingPongNonDet", Correct); }
    #[test] fn sem_one_machine_18() { run_test("Integration/Correct/SEM_OneMachine_18", Correct); }
    #[test] fn sem_one_machine_19() { run_test("Integration/Correct/SEM_OneMachine_19", Correct); }
    #[test] fn sem_one_machine_2() { run_test("Integration/Correct/SEM_OneMachine_2", Correct); }
    #[test] fn sem_one_machine_31() { run_test("Integration/Correct/SEM_OneMachine_31", Correct); }
    #[test] fn sem_one_machine_4() { run_test("Integration/Correct/SEM_OneMachine_4", Correct); }
    #[test] fn sem_one_machine_7() { run_test("Integration/Correct/SEM_OneMachine_7", Correct); }
    #[test] fn sem_two_machines_1() { run_test("Integration/Correct/SEM_TwoMachines_1", Correct); }
    #[test] fn sem_two_machines_14() { run_test("Integration/Correct/SEM_TwoMachines_14", Correct); }
    #[test] fn sem_two_machines_3() { run_test("Integration/Correct/SEM_TwoMachines_3", Correct); }
    #[test] fn sem_two_machines_5() { run_test("Integration/Correct/SEM_TwoMachines_5", Correct); }
    #[test] fn sem_two_machines_7() { run_test("Integration/Correct/SEM_TwoMachines_7", Correct); }
    #[test] fn sem_two_machines_8() { run_test("Integration/Correct/SEM_TwoMachines_8", Correct); }
    #[test] fn test_map_set() { run_test("Integration/Correct/TestMapSet", Correct); }
    #[test] fn token_ring() { run_test("Integration/Correct/TokenRing", Correct); }
    #[test] fn openwsn1() { run_test("Integration/Correct/openwsn1", Correct); }
    #[test] fn ping_pong_exp() { run_test("Integration/Correct/pingPongExp", Correct); }
    #[test] fn ping_pong_foreach() { run_test("Integration/Correct/pingPongForeach", Correct); }
    #[test] fn ping_pong_new() { run_test("Integration/Correct/pingPongNew", Correct); }
    #[test] fn ping_pong_old() { run_test("Integration/Correct/pingPongOld", Correct); }
    #[test] fn ping_pong_receive1() { run_test("Integration/Correct/pingPongReceive1", Correct); }
    #[test] fn ping_pong_receive2() { run_test("Integration/Correct/pingPongReceive2", Correct); }
    #[test] fn ping_pong_receive3() { run_test("Integration/Correct/pingPongReceive3", Correct); }
    #[test] fn ping_pong_receive4() { run_test("Integration/Correct/pingPongReceive4", Correct); }
    #[test] fn ping_pong_tuple() { run_test("Integration/Correct/pingPongTuple", Correct); }
    #[test] fn two_phase_commit() { run_test("Integration/Correct/two-phase-commit", Correct); }
}

mod integration_static_error {
    use super::*;
    #[test] fn token_ring_typos() { run_test("Integration/StaticError/TokenRing_Typos", StaticError); }
}

mod integration_dynamic_error {
    use super::*;
    #[test] fn actions_1() { run_test("Integration/DynamicError/Actions_1", DynamicError); }
    #[test] fn actions_3() { run_test("Integration/DynamicError/Actions_3", DynamicError); }
    #[test] fn actions_5() { run_test("Integration/DynamicError/Actions_5", DynamicError); }
    #[test] fn actions_6() { run_test("Integration/DynamicError/Actions_6", DynamicError); }
    #[test] fn multi_paxos_3() { run_test("Integration/DynamicError/Multi_Paxos_3", DynamicError); }
    #[test] fn multi_paxos_4() { run_test("Integration/DynamicError/Multi_Paxos_4", DynamicError); }
    #[test] fn ping_pong_with_call() { run_test("Integration/DynamicError/PingPongWithCall", DynamicError); }
    #[test] fn sem_one_machine_1() { run_test("Integration/DynamicError/SEM_OneMachine_1", DynamicError); }
    #[test] fn sem_one_machine_3() { run_test("Integration/DynamicError/SEM_OneMachine_3", DynamicError); }
    #[test] fn sem_one_machine_5() { run_test("Integration/DynamicError/SEM_OneMachine_5", DynamicError); }
    #[test] fn sem_one_machine_6() { run_test("Integration/DynamicError/SEM_OneMachine_6", DynamicError); }
    #[test] fn sem_one_machine_8() { run_test("Integration/DynamicError/SEM_OneMachine_8", DynamicError); }
    #[test] fn sem_one_machine_9() { run_test("Integration/DynamicError/SEM_OneMachine_9", DynamicError); }
    #[test] fn sem_one_machine_10() { run_test("Integration/DynamicError/SEM_OneMachine_10", DynamicError); }
    #[test] fn sem_one_machine_11() { run_test("Integration/DynamicError/SEM_OneMachine_11", DynamicError); }
    #[test] fn sem_one_machine_12() { run_test("Integration/DynamicError/SEM_OneMachine_12", DynamicError); }
    #[test] fn sem_one_machine_13() { run_test("Integration/DynamicError/SEM_OneMachine_13", DynamicError); }
    #[test] fn sem_one_machine_14() { run_test("Integration/DynamicError/SEM_OneMachine_14", DynamicError); }
    #[test] fn sem_one_machine_15() { run_test("Integration/DynamicError/SEM_OneMachine_15", DynamicError); }
    #[test] fn sem_one_machine_16() { run_test("Integration/DynamicError/SEM_OneMachine_16", DynamicError); }
    #[test] fn sem_one_machine_20() { run_test("Integration/DynamicError/SEM_OneMachine_20", DynamicError); }
    #[test] fn sem_one_machine_21() { run_test("Integration/DynamicError/SEM_OneMachine_21", DynamicError); }
    #[test] fn sem_one_machine_25() { run_test("Integration/DynamicError/SEM_OneMachine_25", DynamicError); }
    #[test] fn sem_one_machine_28() { run_test("Integration/DynamicError/SEM_OneMachine_28", DynamicError); }
    #[test] fn sem_one_machine_32() { run_test("Integration/DynamicError/SEM_OneMachine_32", DynamicError); }
    #[test] fn sem_one_machine_36() { run_test("Integration/DynamicError/SEM_OneMachine_36", DynamicError); }
    #[test] fn sem_one_machine_37() { run_test("Integration/DynamicError/SEM_OneMachine_37", DynamicError); }
    #[test] fn sem_one_machine_38() { run_test("Integration/DynamicError/SEM_OneMachine_38", DynamicError); }
    #[test] fn sem_one_machine_39() { run_test("Integration/DynamicError/SEM_OneMachine_39", DynamicError); }
    #[test] fn sem_one_machine_40() { run_test("Integration/DynamicError/SEM_OneMachine_40", DynamicError); }
    #[test] fn sem_one_machine_41() { run_test("Integration/DynamicError/SEM_OneMachine_41", DynamicError); }
    #[test] fn sem_one_machine_42() { run_test("Integration/DynamicError/SEM_OneMachine_42", DynamicError); }
    #[test] fn sem_two_machines_2() { run_test("Integration/DynamicError/SEM_TwoMachines_2", DynamicError); }
    #[test] fn sem_two_machines_4() { run_test("Integration/DynamicError/SEM_TwoMachines_4", DynamicError); }
    #[test] fn sem_two_machines_6() { run_test("Integration/DynamicError/SEM_TwoMachines_6", DynamicError); }
    #[test] fn sem_two_machines_9() { run_test("Integration/DynamicError/SEM_TwoMachines_9", DynamicError); }
    #[test] fn sem_two_machines_10() { run_test("Integration/DynamicError/SEM_TwoMachines_10", DynamicError); }
    #[test] fn sem_two_machines_12() { run_test("Integration/DynamicError/SEM_TwoMachines_12", DynamicError); }
    #[test] fn sem_two_machines_15() { run_test("Integration/DynamicError/SEM_TwoMachines_15", DynamicError); }
    #[test] fn sem_two_machines_16() { run_test("Integration/DynamicError/SEM_TwoMachines_16", DynamicError); }
    #[test] fn sem_two_machines_17() { run_test("Integration/DynamicError/SEM_TwoMachines_17", DynamicError); }
    #[test] fn sem_two_machines_18() { run_test("Integration/DynamicError/SEM_TwoMachines_18", DynamicError); }
    #[test] fn sem_two_machines_19() { run_test("Integration/DynamicError/SEM_TwoMachines_19", DynamicError); }
    #[test] fn two_phase_commit_1() { run_test("Integration/DynamicError/two-phase-commit_1", DynamicError); }
}

// =============================================================================
// Liveness — Progress Properties
// =============================================================================

mod liveness_correct {
    use super::*;
    #[test] fn liveness_1() { run_test("Liveness/Correct/Liveness_1", Correct); }
    #[test] fn liveness_1_false_pass() { run_test("Liveness/Correct/Liveness_1_falsePass", Correct); }
    #[test] fn liveness_fair_nondet() { run_test("Liveness/Correct/Liveness_FAIRNONDET", Correct); }
    #[test] fn liveness_fair_nondet2() { run_test("Liveness/Correct/Liveness_FAIRNONDET2", Correct); }
    #[test] fn warm_state_1() { run_test("Liveness/Correct/WarmState_1", Correct); }
}

mod liveness_dynamic_error {
    use super::*;
    #[test] fn infinite_loop_in_atomic_block() { run_test("Liveness/DynamicError/InfiniteLoopInAtomicBlock", DynamicError); }
    #[test] fn liveness_10() { run_test("Liveness/DynamicError/Liveness_10", DynamicError); }
    #[test] fn liveness_1_warm_state() { run_test("Liveness/DynamicError/Liveness_1_WarmState", DynamicError); }
    #[test] fn liveness_2() { run_test("Liveness/DynamicError/Liveness_2", DynamicError); }
    #[test] fn liveness_2_loop_machine_added() { run_test("Liveness/DynamicError/Liveness_2_LoopMachineAdded", DynamicError); }
    #[test] fn liveness_2_bug_found() { run_test("Liveness/DynamicError/Liveness_2_bugFound", DynamicError); }
    #[test] fn liveness_3() { run_test("Liveness/DynamicError/Liveness_3", DynamicError); }
    #[test] fn liveness_4() { run_test("Liveness/DynamicError/Liveness_4", DynamicError); }
    #[test] fn liveness_5() { run_test("Liveness/DynamicError/Liveness_5", DynamicError); }
    #[test] fn liveness_5_deadlock() { run_test("Liveness/DynamicError/Liveness_5_deadlock", DynamicError); }
    #[test] fn liveness_6() { run_test("Liveness/DynamicError/Liveness_6", DynamicError); }
    #[test] fn liveness_6_deadlock() { run_test("Liveness/DynamicError/Liveness_6_deadlock", DynamicError); }
    #[test] fn liveness_7() { run_test("Liveness/DynamicError/Liveness_7", DynamicError); }
    #[test] fn liveness_9() { run_test("Liveness/DynamicError/Liveness_9", DynamicError); }
    #[test] fn liveness_nondet() { run_test("Liveness/DynamicError/Liveness_NONDET", DynamicError); }
    #[test] fn liveness_nondet2() { run_test("Liveness/DynamicError/Liveness_NONDET2", DynamicError); }
}
