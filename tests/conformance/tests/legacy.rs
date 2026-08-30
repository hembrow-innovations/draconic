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

#[test]
fn with_call_fixture_present() {
    assert_fixture_present("es/legacy/with_call");
}

#[test]
fn with_call_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_call");
}

#[test]
fn with_logical_fixture_present() {
    assert_fixture_present("es/legacy/with_logical");
}

#[test]
fn with_logical_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_logical");
}

#[test]
fn with_comparison_fixture_present() {
    assert_fixture_present("es/legacy/with_comparison");
}

#[test]
fn with_comparison_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_comparison");
}

#[test]
fn with_arithmetic_fixture_present() {
    assert_fixture_present("es/legacy/with_arithmetic");
}

#[test]
fn with_arithmetic_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_arithmetic");
}

#[test]
fn with_bitwise_fixture_present() {
    assert_fixture_present("es/legacy/with_bitwise");
}

#[test]
fn with_bitwise_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_bitwise");
}

#[test]
fn with_exponentiation_fixture_present() {
    assert_fixture_present("es/legacy/with_exponentiation");
}

#[test]
fn with_exponentiation_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_exponentiation");
}

#[test]
fn with_conditional_fixture_present() {
    assert_fixture_present("es/legacy/with_conditional");
}

#[test]
fn with_conditional_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_conditional");
}

#[test]
fn with_assignment_fixture_present() {
    assert_fixture_present("es/legacy/with_assignment");
}

#[test]
fn with_assignment_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_assignment");
}

#[test]
fn with_comma_fixture_present() {
    assert_fixture_present("es/legacy/with_comma");
}

#[test]
fn with_comma_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_comma");
}

#[test]
fn with_compound_assign_fixture_present() {
    assert_fixture_present("es/legacy/with_compound_assign");
}

#[test]
fn with_compound_assign_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_compound_assign");
}

#[test]
fn with_object_access_fixture_present() {
    assert_fixture_present("es/legacy/with_object_access");
}

#[test]
fn with_object_access_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_object_access");
}

#[test]
fn with_property_assign_fixture_present() {
    assert_fixture_present("es/legacy/with_property_assign");
}

#[test]
fn with_property_assign_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_property_assign");
}

#[test]
fn with_object_lit_sugar_fixture_present() {
    assert_fixture_present("es/legacy/with_object_lit_sugar");
}

#[test]
fn with_object_lit_sugar_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_object_lit_sugar");
}

#[test]
fn with_array_lit_access_fixture_present() {
    assert_fixture_present("es/legacy/with_array_lit_access");
}

#[test]
fn with_array_lit_access_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_array_lit_access");
}

#[test]
fn with_array_element_assign_fixture_present() {
    assert_fixture_present("es/legacy/with_array_element_assign");
}

#[test]
fn with_array_element_assign_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_array_element_assign");
}

#[test]
fn with_array_spread_fixture_present() {
    assert_fixture_present("es/legacy/with_array_spread");
}

#[test]
fn with_array_spread_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_array_spread");
}

#[test]
fn with_call_spread_fixture_present() {
    assert_fixture_present("es/legacy/with_call_spread");
}

#[test]
fn with_call_spread_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_call_spread");
}

#[test]
fn with_string_lit_access_fixture_present() {
    assert_fixture_present("es/legacy/with_string_lit_access");
}

#[test]
fn with_string_lit_access_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_string_lit_access");
}

#[test]
fn with_template_lit_fixture_present() {
    assert_fixture_present("es/legacy/with_template_lit");
}

#[test]
fn with_template_lit_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_template_lit");
}

#[test]
fn with_unicode_escapes_fixture_present() {
    assert_fixture_present("es/legacy/with_unicode_escapes");
}

#[test]
fn with_unicode_escapes_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_unicode_escapes");
}

#[test]
fn with_utf16_semantics_fixture_present() {
    assert_fixture_present("es/legacy/with_utf16_semantics");
}

#[test]
fn with_utf16_semantics_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_utf16_semantics");
}

#[test]
fn with_number_literals_fixture_present() {
    assert_fixture_present("es/legacy/with_number_literals");
}

#[test]
fn with_number_literals_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_number_literals");
}

#[test]
fn with_bigint_literals_fixture_present() {
    assert_fixture_present("es/legacy/with_bigint_literals");
}

#[test]
fn with_bigint_literals_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_bigint_literals");
}

#[test]
fn with_bigint_ops_fixture_present() {
    assert_fixture_present("es/legacy/with_bigint_ops");
}

#[test]
fn with_bigint_ops_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_bigint_ops");
}

#[test]
fn with_bigint_pow_fixture_present() {
    assert_fixture_present("es/legacy/with_bigint_pow");
}

#[test]
fn with_bigint_pow_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_bigint_pow");
}

#[test]
fn with_math_fixture_present() {
    assert_fixture_present("es/legacy/with_math");
}

#[test]
fn with_math_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_math");
}

#[test]
fn with_number_global_fixture_present() {
    assert_fixture_present("es/legacy/with_number_global");
}

#[test]
fn with_number_global_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_number_global");
}

#[test]
fn with_symbol_basics_fixture_present() {
    assert_fixture_present("es/legacy/with_symbol_basics");
}

#[test]
fn with_symbol_basics_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_symbol_basics");
}

#[test]
fn with_symbol_property_keys_fixture_present() {
    assert_fixture_present("es/legacy/with_symbol_property_keys");
}

#[test]
fn with_symbol_property_keys_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_symbol_property_keys");
}

#[test]
fn with_abstract_eq_coercion_fixture_present() {
    assert_fixture_present("es/legacy/with_abstract_eq_coercion");
}

#[test]
fn with_abstract_eq_coercion_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_abstract_eq_coercion");
}

#[test]
fn with_to_primitive_fixture_present() {
    assert_fixture_present("es/legacy/with_to_primitive");
}

#[test]
fn with_to_primitive_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_to_primitive");
}

#[test]
fn with_global_basics_fixture_present() {
    assert_fixture_present("es/legacy/with_global_basics");
}

#[test]
fn with_global_basics_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_global_basics");
}

#[test]
fn with_error_ctors_fixture_present() {
    assert_fixture_present("es/legacy/with_error_ctors");
}

#[test]
fn with_error_ctors_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_error_ctors");
}

#[test]
fn with_global_functions_fixture_present() {
    assert_fixture_present("es/legacy/with_global_functions");
}

#[test]
fn with_global_functions_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_global_functions");
}

#[test]
fn with_uri_functions_fixture_present() {
    assert_fixture_present("es/legacy/with_uri_functions");
}

#[test]
fn with_uri_functions_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_uri_functions");
}

#[test]
fn with_json_fixture_present() {
    assert_fixture_present("es/legacy/with_json");
}

#[test]
fn with_json_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_json");
}

#[test]
fn with_date_fixture_present() {
    assert_fixture_present("es/legacy/with_date");
}

#[test]
fn with_date_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_date");
}

#[test]
fn with_regexp_fixture_present() {
    assert_fixture_present("es/legacy/with_regexp");
}

#[test]
fn with_regexp_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_regexp");
}

#[test]
fn with_map_set_fixture_present() {
    assert_fixture_present("es/legacy/with_map_set");
}

#[test]
fn with_map_set_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_map_set");
}

#[test]
fn with_weak_map_set_fixture_present() {
    assert_fixture_present("es/legacy/with_weak_map_set");
}

#[test]
fn with_weak_map_set_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_weak_map_set");
}

#[test]
fn with_arraybuffer_typedarrays_fixture_present() {
    assert_fixture_present("es/legacy/with_arraybuffer_typedarrays");
}

#[test]
fn with_arraybuffer_typedarrays_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_arraybuffer_typedarrays");
}

#[test]
fn with_promise_basics_fixture_present() {
    assert_fixture_present("es/legacy/with_promise_basics");
}

#[test]
fn with_promise_basics_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_promise_basics");
}

#[test]
fn with_promise_statics_fixture_present() {
    assert_fixture_present("es/legacy/with_promise_statics");
}

#[test]
fn with_promise_statics_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_promise_statics");
}

#[test]
fn with_promise_finally_fixture_present() {
    assert_fixture_present("es/legacy/with_promise_finally");
}

#[test]
fn with_promise_finally_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_promise_finally");
}

#[test]
fn with_promise_all_fixture_present() {
    assert_fixture_present("es/legacy/with_promise_all");
}

#[test]
fn with_promise_all_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_promise_all");
}

#[test]
fn with_promise_race_fixture_present() {
    assert_fixture_present("es/legacy/with_promise_race");
}

#[test]
fn with_promise_race_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_promise_race");
}

#[test]
fn with_promise_all_settled_fixture_present() {
    assert_fixture_present("es/legacy/with_promise_all_settled");
}

#[test]
fn with_promise_all_settled_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_promise_all_settled");
}

#[test]
fn with_promise_any_fixture_present() {
    assert_fixture_present("es/legacy/with_promise_any");
}

#[test]
fn with_promise_any_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_promise_any");
}

#[test]
fn with_proxy_basics_fixture_present() {
    assert_fixture_present("es/legacy/with_proxy_basics");
}

#[test]
fn with_proxy_basics_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_proxy_basics");
}

#[test]
fn with_proxy_set_fixture_present() {
    assert_fixture_present("es/legacy/with_proxy_set");
}

#[test]
fn with_proxy_set_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_proxy_set");
}

#[test]
fn with_proxy_has_fixture_present() {
    assert_fixture_present("es/legacy/with_proxy_has");
}

#[test]
fn with_proxy_has_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_proxy_has");
}

#[test]
fn with_proxy_delete_fixture_present() {
    assert_fixture_present("es/legacy/with_proxy_delete");
}

#[test]
fn with_proxy_delete_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_proxy_delete");
}

#[test]
fn with_proxy_apply_fixture_present() {
    assert_fixture_present("es/legacy/with_proxy_apply");
}

#[test]
fn with_proxy_apply_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_proxy_apply");
}

#[test]
fn with_proxy_construct_fixture_present() {
    assert_fixture_present("es/legacy/with_proxy_construct");
}

#[test]
fn with_proxy_construct_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_proxy_construct");
}

#[test]
fn with_proxy_own_keys_fixture_present() {
    assert_fixture_present("es/legacy/with_proxy_own_keys");
}

#[test]
fn with_proxy_own_keys_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_proxy_own_keys");
}

#[test]
fn with_proxy_prototype_fixture_present() {
    assert_fixture_present("es/legacy/with_proxy_prototype");
}

#[test]
fn with_proxy_prototype_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_proxy_prototype");
}

#[test]
fn with_proxy_define_property_fixture_present() {
    assert_fixture_present("es/legacy/with_proxy_define_property");
}

#[test]
fn with_proxy_define_property_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_proxy_define_property");
}

#[test]
fn with_proxy_extensible_fixture_present() {
    assert_fixture_present("es/legacy/with_proxy_extensible");
}

#[test]
fn with_proxy_extensible_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_proxy_extensible");
}

#[test]
fn with_escape_unescape_fixture_present() {
    assert_fixture_present("es/legacy/with_escape_unescape");
}

#[test]
fn with_escape_unescape_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_escape_unescape");
}

#[test]
fn with_object_proto_fixture_present() {
    assert_fixture_present("es/legacy/with_object_proto");
}

#[test]
fn with_object_proto_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_object_proto");
}

#[test]
fn with_string_proto_annex_fixture_present() {
    assert_fixture_present("es/legacy/with_string_proto_annex");
}

#[test]
fn with_string_proto_annex_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_string_proto_annex");
}

#[test]
fn with_date_proto_annex_fixture_present() {
    assert_fixture_present("es/legacy/with_date_proto_annex");
}

#[test]
fn with_date_proto_annex_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_date_proto_annex");
}

#[test]
fn with_regexp_compile_fixture_present() {
    assert_fixture_present("es/legacy/with_regexp_compile");
}

#[test]
fn with_regexp_compile_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_regexp_compile");
}

#[test]
fn with_string_trim_left_right_fixture_present() {
    assert_fixture_present("es/legacy/with_string_trim_left_right");
}

#[test]
fn with_string_trim_left_right_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_string_trim_left_right");
}

#[test]
fn with_object_accessor_legacy_fixture_present() {
    assert_fixture_present("es/legacy/with_object_accessor_legacy");
}

#[test]
fn with_object_accessor_legacy_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_object_accessor_legacy");
}

#[test]
fn with_html_comments_fixture_present() {
    assert_fixture_present("es/legacy/with_html_comments");
}

#[test]
fn with_html_comments_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_html_comments");
}

#[test]
fn with_legacy_octal_string_fixture_present() {
    assert_fixture_present("es/legacy/with_legacy_octal_string");
}

#[test]
fn with_legacy_octal_string_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_legacy_octal_string");
}

#[test]
fn with_legacy_octal_numeric_fixture_present() {
    assert_fixture_present("es/legacy/with_legacy_octal_numeric");
}

#[test]
fn with_legacy_octal_numeric_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_legacy_octal_numeric");
}

#[test]
fn with_labelled_function_fixture_present() {
    assert_fixture_present("es/legacy/with_labelled_function");
}

#[test]
fn with_labelled_function_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_labelled_function");
}

#[test]
fn with_if_function_fixture_present() {
    assert_fixture_present("es/legacy/with_if_function");
}

#[test]
fn with_if_function_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_if_function");
}

#[test]
fn with_block_function_fixture_present() {
    assert_fixture_present("es/legacy/with_block_function");
}

#[test]
fn with_block_function_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_block_function");
}

#[test]
fn with_var_decl_fixture_present() {
    assert_fixture_present("es/legacy/with_var_decl");
}

#[test]
fn with_var_decl_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_var_decl");
}

#[test]
fn with_var_for_fixture_present() {
    assert_fixture_present("es/legacy/with_var_for");
}

#[test]
fn with_var_for_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_var_for");
}

#[test]
fn with_regexp_statics_fixture_present() {
    assert_fixture_present("es/legacy/with_regexp_statics");
}

#[test]
fn with_regexp_statics_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_regexp_statics");
}

#[test]
fn with_var_catch_fixture_present() {
    assert_fixture_present("es/legacy/with_var_catch");
}

#[test]
fn with_var_catch_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_var_catch");
}

#[test]
fn with_regexp_literal_fixture_present() {
    assert_fixture_present("es/legacy/with_regexp_literal");
}

#[test]
fn with_regexp_literal_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_regexp_literal");
}

#[test]
fn with_object_destructure_fixture_present() {
    assert_fixture_present("es/legacy/with_object_destructure");
}

#[test]
fn with_object_destructure_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_object_destructure");
}

#[test]
fn with_destructure_defaults_fixture_present() {
    assert_fixture_present("es/legacy/with_destructure_defaults");
}

#[test]
fn with_destructure_defaults_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_destructure_defaults");
}

#[test]
fn with_accessors_fixture_present() {
    assert_fixture_present("es/legacy/with_accessors");
}

#[test]
fn with_accessors_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_accessors");
}

#[test]
fn with_param_destructure_fixture_present() {
    assert_fixture_present("es/legacy/with_param_destructure");
}

#[test]
fn with_param_destructure_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_param_destructure");
}

#[test]
fn with_class_fields_fixture_present() {
    assert_fixture_present("es/legacy/with_class_fields");
}

#[test]
fn with_class_fields_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_class_fields");
}

#[test]
fn with_new_target_fixture_present() {
    assert_fixture_present("es/legacy/with_new_target");
}

#[test]
fn with_new_target_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_new_target");
}

#[test]
fn with_object_spread_fixture_present() {
    assert_fixture_present("es/legacy/with_object_spread");
}

#[test]
fn with_object_spread_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_object_spread");
}

#[test]
fn with_private_fields_fixture_present() {
    assert_fixture_present("es/legacy/with_private_fields");
}

#[test]
fn with_private_fields_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_private_fields");
}

#[test]
fn with_static_private_fields_fixture_present() {
    assert_fixture_present("es/legacy/with_static_private_fields");
}

#[test]
fn with_static_private_fields_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_static_private_fields");
}

#[test]
fn with_private_methods_fixture_present() {
    assert_fixture_present("es/legacy/with_private_methods");
}

#[test]
fn with_private_methods_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_private_methods");
}

#[test]
fn with_static_private_methods_fixture_present() {
    assert_fixture_present("es/legacy/with_static_private_methods");
}

#[test]
fn with_static_private_methods_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_static_private_methods");
}

#[test]
fn with_private_accessors_fixture_present() {
    assert_fixture_present("es/legacy/with_private_accessors");
}

#[test]
fn with_private_accessors_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_private_accessors");
}

#[test]
fn with_private_in_fixture_present() {
    assert_fixture_present("es/legacy/with_private_in");
}

#[test]
fn with_private_in_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_private_in");
}

#[test]
fn with_static_blocks_fixture_present() {
    assert_fixture_present("es/legacy/with_static_blocks");
}

#[test]
fn with_static_blocks_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_static_blocks");
}

#[test]
fn with_async_generators_fixture_present() {
    assert_fixture_present("es/legacy/with_async_generators");
}

#[test]
fn with_async_generators_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_async_generators");
}

#[test]
fn with_dynamic_import_fixture_present() {
    assert_fixture_present("es/legacy/with_dynamic_import");
}

#[test]
fn with_dynamic_import_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_dynamic_import");
}

#[test]
fn with_import_defer_source_fixture_present() {
    assert_fixture_present("es/legacy/with_import_defer_source");
}

#[test]
fn with_import_defer_source_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_import_defer_source");
}

#[test]
fn with_yield_ident_fixture_present() {
    assert_fixture_present("es/legacy/with_yield_ident");
}

#[test]
fn with_yield_ident_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_yield_ident");
}

#[test]
fn with_import_attributes_fixture_present() {
    assert_fixture_present("es/legacy/with_import_attributes");
}

#[test]
fn with_import_attributes_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_import_attributes");
}

#[test]
fn with_using_fixture_present() {
    assert_fixture_present("es/legacy/with_using");
}

#[test]
fn with_using_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_using");
}

#[test]
fn with_reserved_methods_fixture_present() {
    assert_fixture_present("es/legacy/with_reserved_methods");
}

#[test]
fn with_reserved_methods_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_reserved_methods");
}

#[test]
fn with_await_ident_fixture_present() {
    assert_fixture_present("es/legacy/with_await_ident");
}

#[test]
fn with_await_ident_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_await_ident");
}

#[test]
fn with_class_elements_residual_fixture_present() {
    assert_fixture_present("es/legacy/with_class_elements_residual");
}

#[test]
fn with_class_elements_residual_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_class_elements_residual");
}

#[test]
fn with_async_methods_fixture_present() {
    assert_fixture_present("es/legacy/with_async_methods");
}

#[test]
fn with_async_methods_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_async_methods");
}

#[test]
fn with_class_expr_fixture_present() {
    assert_fixture_present("es/legacy/with_class_expr");
}

#[test]
fn with_class_expr_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_class_expr");
}

#[test]
fn with_export_class_fixture_present() {
    assert_fixture_present("es/legacy/with_export_class");
}

#[test]
fn with_export_class_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_export_class");
}

#[test]
fn with_export_ns_from_fixture_present() {
    assert_fixture_present("es/legacy/with_export_ns_from");
}

#[test]
fn with_export_ns_from_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_export_ns_from");
}

#[test]
fn with_export_named_from_fixture_present() {
    assert_fixture_present("es/legacy/with_export_named_from");
}

#[test]
fn with_export_named_from_runs() {
    assert_fixture_runs_declared_targets("es/legacy/with_export_named_from");
}
