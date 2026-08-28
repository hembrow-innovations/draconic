//! ROADMAP E17: non-strict legacy fixtures on declared targets.

use draconic_conformance::{fixtures_dir, load_fixtures, run_fixture};

fn assert_fixture_present(id: &str) {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|x| *x == id),
        "missing {id} fixture, got {ids:?}"
    );
}

fn assert_fixture_runs_declared_targets(id: &str) {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures.iter().find(|f| f.id == id).expect(id);
    assert!(
        !fixture.targets.is_empty(),
        "{id} must declare at least one target"
    );
    for r in run_fixture(fixture) {
        assert!(
            r.ok,
            "{} @ {}: {}",
            r.fixture_id,
            r.target.as_str(),
            r.message
        );
    }
}

#[test]
fn with_basic_fixture_present() {
    assert_fixture_present("es/legacy/with_basic");
}

#[test]
fn with_basic_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_basic");
}

#[test]
fn with_nested_fixture_present() {
    assert_fixture_present("es/legacy/with_nested");
}

#[test]
fn with_nested_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_nested");
}

#[test]
fn arguments_callee_fixture_present() {
    assert_fixture_present("es/legacy/arguments_callee");
}

#[test]
fn arguments_callee_runs() {
    assert_fixture_runs_declared_targets("es/legacy/arguments_callee");
}

#[test]
fn arguments_mapped_fixture_present() {
    assert_fixture_present("es/legacy/arguments_mapped");
}

#[test]
fn arguments_mapped_runs() {
    assert_fixture_runs_declared_targets("es/legacy/arguments_mapped");
}

#[test]
fn delete_identifier_fixture_present() {
    assert_fixture_present("es/legacy/delete_identifier");
}

#[test]
fn delete_identifier_runs() {
    assert_fixture_runs_declared_targets("es/legacy/delete_identifier");
}

#[test]
fn duplicate_params_fixture_present() {
    assert_fixture_present("es/legacy/duplicate_params");
}

#[test]
fn duplicate_params_runs() {
    assert_fixture_runs_declared_targets("es/legacy/duplicate_params");
}

#[test]
fn function_caller_arguments_fixture_present() {
    assert_fixture_present("es/legacy/function_caller_arguments");
}

#[test]
fn function_caller_arguments_runs() {
    assert_fixture_runs_declared_targets("es/legacy/function_caller_arguments");
}

#[test]
fn sloppy_this_fixture_present() {
    assert_fixture_present("es/legacy/sloppy_this");
}

#[test]
fn sloppy_this_runs() {
    assert_fixture_runs_declared_targets("es/legacy/sloppy_this");
}

#[test]
fn implicit_global_fixture_present() {
    assert_fixture_present("es/legacy/implicit_global");
}

#[test]
fn implicit_global_runs() {
    assert_fixture_runs_declared_targets("es/legacy/implicit_global");
}

#[test]
fn future_reserved_idents_fixture_present() {
    assert_fixture_present("es/legacy/future_reserved_idents");
}

#[test]
fn future_reserved_idents_runs() {
    assert_fixture_runs_declared_targets("es/legacy/future_reserved_idents");
}

#[test]
fn for_in_of_implicit_global_fixture_present() {
    assert_fixture_present("es/legacy/for_in_of_implicit_global");
}

#[test]
fn for_in_of_implicit_global_runs() {
    assert_fixture_runs_declared_targets("es/legacy/for_in_of_implicit_global");
}

#[test]
fn eval_arguments_idents_fixture_present() {
    assert_fixture_present("es/legacy/eval_arguments_idents");
}

#[test]
fn eval_arguments_idents_runs() {
    assert_fixture_runs_declared_targets("es/legacy/eval_arguments_idents");
}

#[test]
fn eval_var_inject_fixture_present() {
    assert_fixture_present("es/legacy/eval_var_inject");
}

#[test]
fn eval_var_inject_runs() {
    assert_fixture_runs_declared_targets("es/legacy/eval_var_inject");
}

#[test]
fn arguments_mapped_residual_fixture_present() {
    assert_fixture_present("es/legacy/arguments_mapped_residual");
}

#[test]
fn arguments_mapped_residual_runs() {
    assert_fixture_runs_declared_targets("es/legacy/arguments_mapped_residual");
}

#[test]
fn arguments_callee_residual_fixture_present() {
    assert_fixture_present("es/legacy/arguments_callee_residual");
}

#[test]
fn arguments_callee_residual_runs() {
    assert_fixture_runs_declared_targets("es/legacy/arguments_callee_residual");
}

#[test]
fn putvalue_delete_property_fixture_present() {
    assert_fixture_present("es/legacy/putvalue_delete_property");
}

#[test]
fn putvalue_delete_property_runs() {
    assert_fixture_runs_declared_targets("es/legacy/putvalue_delete_property");
}

#[test]
fn putvalue_accessor_proto_fixture_present() {
    assert_fixture_present("es/legacy/putvalue_accessor_proto");
}

#[test]
fn putvalue_accessor_proto_runs() {
    assert_fixture_runs_declared_targets("es/legacy/putvalue_accessor_proto");
}

#[test]
fn putvalue_immutable_globals_fixture_present() {
    assert_fixture_present("es/legacy/putvalue_immutable_globals");
}

#[test]
fn putvalue_immutable_globals_runs() {
    assert_fixture_runs_declared_targets("es/legacy/putvalue_immutable_globals");
}

#[test]
fn arguments_unmapped_nonsimple_fixture_present() {
    assert_fixture_present("es/legacy/arguments_unmapped_nonsimple");
}

#[test]
fn arguments_unmapped_nonsimple_runs() {
    assert_fixture_runs_declared_targets("es/legacy/arguments_unmapped_nonsimple");
}

#[test]
fn arguments_define_property_fixture_present() {
    assert_fixture_present("es/legacy/arguments_define_property");
}

#[test]
fn arguments_define_property_runs() {
    assert_fixture_runs_declared_targets("es/legacy/arguments_define_property");
}

#[test]
fn arguments_exotic_residual_fixture_present() {
    assert_fixture_present("es/legacy/arguments_exotic_residual");
}

#[test]
fn arguments_exotic_residual_runs() {
    assert_fixture_runs_declared_targets("es/legacy/arguments_exotic_residual");
}

#[test]
fn arguments_array_mutators_fixture_present() {
    assert_fixture_present("es/legacy/arguments_array_mutators");
}

#[test]
fn arguments_array_mutators_runs() {
    assert_fixture_runs_declared_targets("es/legacy/arguments_array_mutators");
}

#[test]
fn arguments_mutators_freeze_fixture_present() {
    assert_fixture_present("es/legacy/arguments_mutators_freeze");
}

#[test]
fn arguments_mutators_freeze_runs() {
    assert_fixture_runs_declared_targets("es/legacy/arguments_mutators_freeze");
}

#[test]
fn arguments_reflect_iter_fixture_present() {
    assert_fixture_present("es/legacy/arguments_reflect_iter");
}

#[test]
fn arguments_reflect_iter_runs() {
    assert_fixture_runs_declared_targets("es/legacy/arguments_reflect_iter");
}

#[test]
fn with_residual_fixture_present() {
    assert_fixture_present("es/legacy/with_residual");
}

#[test]
fn with_residual_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_residual");
}

#[test]
fn function_ctor_sloppy_fixture_present() {
    assert_fixture_present("es/legacy/function_ctor_sloppy");
}

#[test]
fn function_ctor_sloppy_runs() {
    assert_fixture_runs_declared_targets("es/legacy/function_ctor_sloppy");
}

#[test]
fn eval_this_residual_fixture_present() {
    assert_fixture_present("es/legacy/eval_this_residual");
}

#[test]
fn eval_this_residual_runs() {
    assert_fixture_runs_declared_targets("es/legacy/eval_this_residual");
}

#[test]
fn eval_strict_source_fixture_present() {
    assert_fixture_present("es/legacy/eval_strict_source");
}

#[test]
fn eval_strict_source_runs() {
    assert_fixture_runs_declared_targets("es/legacy/eval_strict_source");
}

#[test]
fn eval_in_with_fixture_present() {
    assert_fixture_present("es/legacy/eval_in_with");
}

#[test]
fn eval_in_with_runs() {
    assert_fixture_runs_declared_targets("es/legacy/eval_in_with");
}

#[test]
fn eval_arguments_fixture_present() {
    assert_fixture_present("es/legacy/eval_arguments");
}

#[test]
fn eval_arguments_runs() {
    assert_fixture_runs_declared_targets("es/legacy/eval_arguments");
}

#[test]
fn eval_var_delete_fixture_present() {
    assert_fixture_present("es/legacy/eval_var_delete");
}

#[test]
fn eval_var_delete_runs() {
    assert_fixture_runs_declared_targets("es/legacy/eval_var_delete");
}

#[test]
fn with_unscopables_fixture_present() {
    assert_fixture_present("es/legacy/with_unscopables");
}

#[test]
fn with_unscopables_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_unscopables");
}

#[test]
fn with_proxy_fixture_present() {
    assert_fixture_present("es/legacy/with_proxy");
}

#[test]
fn with_proxy_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_proxy");
}

#[test]
fn with_for_update_fixture_present() {
    assert_fixture_present("es/legacy/with_for_update");
}

#[test]
fn with_for_update_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_for_update");
}

#[test]
fn with_proto_fixture_present() {
    assert_fixture_present("es/legacy/with_proto");
}

#[test]
fn with_proto_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_proto");
}

#[test]
fn with_logical_assign_fixture_present() {
    assert_fixture_present("es/legacy/with_logical_assign");
}

#[test]
fn with_logical_assign_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_logical_assign");
}

#[test]
fn with_destructure_fixture_present() {
    assert_fixture_present("es/legacy/with_destructure");
}

#[test]
fn with_destructure_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_destructure");
}

#[test]
fn with_lexical_fixture_present() {
    assert_fixture_present("es/legacy/with_lexical");
}

#[test]
fn with_lexical_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_lexical");
}

#[test]
fn with_hoist_fixture_present() {
    assert_fixture_present("es/legacy/with_hoist");
}

#[test]
fn with_hoist_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_hoist");
}

#[test]
fn with_try_catch_fixture_present() {
    assert_fixture_present("es/legacy/with_try_catch");
}

#[test]
fn with_try_catch_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_try_catch");
}

#[test]
fn with_closures_fixture_present() {
    assert_fixture_present("es/legacy/with_closures");
}

#[test]
fn with_closures_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_closures");
}

#[test]
fn with_generators_fixture_present() {
    assert_fixture_present("es/legacy/with_generators");
}

#[test]
fn with_generators_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_generators");
}

#[test]
fn with_async_fixture_present() {
    assert_fixture_present("es/legacy/with_async");
}

#[test]
fn with_async_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_async");
}

#[test]
fn with_classes_fixture_present() {
    assert_fixture_present("es/legacy/with_classes");
}

#[test]
fn with_classes_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_classes");
}

#[test]
fn with_function_ctor_fixture_present() {
    assert_fixture_present("es/legacy/with_function_ctor");
}

#[test]
fn with_function_ctor_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_function_ctor");
}

#[test]
fn with_tagged_template_fixture_present() {
    assert_fixture_present("es/legacy/with_tagged_template");
}

#[test]
fn with_tagged_template_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_tagged_template");
}

#[test]
fn with_new_fixture_present() {
    assert_fixture_present("es/legacy/with_new");
}

#[test]
fn with_new_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_new");
}

#[test]
fn with_optional_chain_fixture_present() {
    assert_fixture_present("es/legacy/with_optional_chain");
}

#[test]
fn with_optional_chain_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_optional_chain");
}

#[test]
fn with_typeof_fixture_present() {
    assert_fixture_present("es/legacy/with_typeof");
}

#[test]
fn with_typeof_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_typeof");
}

#[test]
fn with_switch_fixture_present() {
    assert_fixture_present("es/legacy/with_switch");
}

#[test]
fn with_switch_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_switch");
}

#[test]
fn with_if_fixture_present() {
    assert_fixture_present("es/legacy/with_if");
}

#[test]
fn with_if_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_if");
}

#[test]
fn with_while_fixture_present() {
    assert_fixture_present("es/legacy/with_while");
}

#[test]
fn with_while_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_while");
}

#[test]
fn with_for_fixture_present() {
    assert_fixture_present("es/legacy/with_for");
}

#[test]
fn with_for_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_for");
}

#[test]
fn with_labeled_fixture_present() {
    assert_fixture_present("es/legacy/with_labeled");
}

#[test]
fn with_labeled_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_labeled");
}

#[test]
fn with_for_await_fixture_present() {
    assert_fixture_present("es/legacy/with_for_await");
}

#[test]
fn with_for_await_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_for_await");
}

#[test]
fn with_in_instanceof_fixture_present() {
    assert_fixture_present("es/legacy/with_in_instanceof");
}

#[test]
fn with_in_instanceof_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_in_instanceof");
}

#[test]
fn with_void_fixture_present() {
    assert_fixture_present("es/legacy/with_void");
}

#[test]
fn with_void_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_void");
}

#[test]
fn with_delete_fixture_present() {
    assert_fixture_present("es/legacy/with_delete");
}

#[test]
fn with_delete_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_delete");
}
