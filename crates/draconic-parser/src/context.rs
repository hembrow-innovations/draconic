/// Grammar parameters pushed and popped as one value.
#[derive(Clone)]
pub(crate) struct ParserContext {
    /// When false, relational `in` is not parsed (for-header left-hand side).
    pub(crate) allow_in: bool,
    /// YieldExpression context (`function*`, generator methods). When false and
    /// non-strict, `yield` is an IdentifierReference / BindingIdentifier (E19.37).
    pub(crate) in_generator: bool,
    /// `[+Await]` grammar parameter: modules, async functions, class static blocks.
    /// When false, `await` is IdentifierReference / BindingIdentifier (E19.52).
    pub(crate) in_await_context: bool,
    /// Strict mode (directive prologue, class bodies). `yield` is reserved.
    pub(crate) in_strict: bool,
    /// Module goal (top-level `using` / `await using` allowed).
    pub(crate) is_module: bool,
    /// Nesting depth of Block / function body (Script `using` early error).
    pub(crate) using_container_depth: u32,
    /// True while parsing a CaseClause/DefaultClause StatementList directly
    /// (nested blocks clear this; using is forbidden in the case list itself).
    pub(crate) forbid_direct_using: bool,
    /// Stack of private names for nested classes (E19.36 / E19.39 inheritance).
    /// Outer class names are visible inside nested class bodies.
    pub(crate) class_private_stack: Vec<Vec<String>>,
    /// Depth of non-arrow functions / methods / static blocks (E19.67 `new.target`).
    /// Arrows are transparent for Contains NewTarget.
    pub(crate) new_target_depth: u32,
    /// Depth of method/constructor bodies where SuperProperty is allowed (E19.67).
    pub(crate) super_property_depth: u32,
    /// Directive prologue saw a string with Annex B legacy octal escape (E19.69).
    pub(crate) prologue_had_legacy_escape: bool,
}

impl ParserContext {
    pub(crate) fn new(is_module: bool) -> Self {
        Self {
            allow_in: true,
            in_generator: false,
            in_await_context: is_module,
            in_strict: false,
            is_module,
            using_container_depth: 0,
            forbid_direct_using: false,
            class_private_stack: Vec::new(),
            new_target_depth: 0,
            super_property_depth: 0,
            prologue_had_legacy_escape: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn script_context_starts_without_await() {
        let ctx = ParserContext::new(false);
        assert!(ctx.allow_in);
        assert!(!ctx.in_generator);
        assert!(!ctx.in_await_context);
        assert!(!ctx.in_strict);
        assert!(!ctx.is_module);
        assert_eq!(ctx.using_container_depth, 0);
        assert!(!ctx.forbid_direct_using);
        assert!(ctx.class_private_stack.is_empty());
        assert_eq!(ctx.new_target_depth, 0);
        assert_eq!(ctx.super_property_depth, 0);
        assert!(!ctx.prologue_had_legacy_escape);
    }

    #[test]
    fn module_context_starts_in_await() {
        let ctx = ParserContext::new(true);
        assert!(ctx.is_module);
        assert!(ctx.in_await_context);
        assert!(!ctx.in_strict);
        assert!(ctx.allow_in);
    }

    #[test]
    fn clone_restore_keeps_unrelated_flags() {
        let mut ctx = ParserContext::new(false);
        let saved = ctx.clone();
        ctx.allow_in = false;
        ctx.in_generator = true;
        ctx.in_await_context = true;
        ctx.in_strict = true;
        ctx.using_container_depth = 3;
        ctx.forbid_direct_using = true;
        ctx.class_private_stack.push(vec!["#x".into()]);
        ctx.new_target_depth = 2;
        ctx.super_property_depth = 1;
        ctx.prologue_had_legacy_escape = true;
        ctx = saved;
        assert!(ctx.allow_in);
        assert!(!ctx.in_generator);
        assert!(!ctx.in_await_context);
        assert!(!ctx.in_strict);
        assert_eq!(ctx.using_container_depth, 0);
        assert!(!ctx.forbid_direct_using);
        assert!(ctx.class_private_stack.is_empty());
        assert_eq!(ctx.new_target_depth, 0);
        assert_eq!(ctx.super_property_depth, 0);
        assert!(!ctx.prologue_had_legacy_escape);
    }
}
