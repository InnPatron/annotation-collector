use rustc_ast::ast::Attribute;
use rustc_lint::{EarlyContext, EarlyLintPass, LintPass};
use rustc_session::{declare_lint_pass, declare_tool_lint};

declare_tool_lint!(pub smpl::SMPL_LINT, Allow, "smpl::lint description", true);

declare_lint_pass!(SmplLint => [&SMPL_LINT]);

impl EarlyLintPass for SmplLint {
    fn enter_lint_attrs(&mut self, _: &EarlyContext<'_>, attrs: &[Attribute]) {
        for a in attrs {
            dbg!(a);
        }
    }
}
