#![feature(rustc_private)]

// NOTE: For the example to compile, you will need to first run the following:
//   rustup component add rustc-dev
//
// NOTE: May also need to install the llmv-preview component as well
//
extern crate rustc_ast;
extern crate rustc_error_codes;
extern crate rustc_errors;
extern crate rustc_feature;
extern crate rustc_hash;
extern crate rustc_hir;
extern crate rustc_interface;
extern crate rustc_lint;
extern crate rustc_middle;
extern crate rustc_session;
extern crate rustc_span;

mod lint;

use rustc_ast::ast;
use rustc_errors::registry;
use rustc_hash::{FxHashMap, FxHashSet};
use rustc_lint::{LintId, LintStore};
use rustc_session::{config, Session};
use rustc_span::{source_map, symbol};

use std::path;
use std::process;
use std::str;

use self::lint::{SmplLint, SMPL_LINT};

fn register_lints(_: &Session, lint_store: &mut LintStore) {
    lint_store.register_lints(&[SMPL_LINT]);
    lint_store.register_group(
        true,
        "smpl::lint",
        Some("smpl"),
        vec![LintId::of(SMPL_LINT)],
    );
    lint_store.register_pre_expansion_pass(|| Box::new(SmplLint));
}

fn pretty_path<T: Iterator<Item = impl std::fmt::Display>>(mut t: T) -> String {
    let mut buffer = String::new();
    let i = t.next().unwrap();
    buffer.push_str(&format!("{}", i));
    for i in t {
        buffer.push_str(&format!("::{}", i));
    }

    buffer
}

// MacArgs: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_ast/ast/enum.MacArgs.html
// TokenStream: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_ast/tokenstream/struct.TokenStream.html
// TokenTree: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_ast/tokenstream/enum.TokenTree.html
// Token: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_ast/token/struct.Token.html
// Lit: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_ast/token/struct.Lit.html
fn pretty_macro_args(args: &rustc_ast::ast::MacArgs) -> String {
    use rustc_ast::ast::MacArgs;
    use rustc_ast::token::{LitKind, TokenKind};
    use rustc_ast::tokenstream::TokenTree;

    let mut buffer = String::new();
    let mut found = false;

    let tstream = match args {
        MacArgs::Empty => return "EMPTY".to_string(),
        MacArgs::Delimited(_, _, ref tstream) => tstream,
        MacArgs::Eq(..) => return "Found EQ attribute form".to_string(),
    };
    for token_tree in tstream.trees_ref() {
        let token = match token_tree {
            TokenTree::Token(ref t) => t,
            TokenTree::Delimited(..) => continue,
        };

        let literal = match token.kind {
            TokenKind::Literal(ref lit) => lit,
            _ => continue,
        };

        match literal.kind {
            LitKind::Str => {
                buffer.push_str(&format!("\"{}\"", literal.symbol));
                found = true;
            }

            _ => continue,
        }
    }

    if found {
        buffer
    } else {
        "Unknown argument".to_string()
    }
}

fn is_smpl_item(item: &rustc_hir::Item) -> bool {
    // Item: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_hir/struct.Item.html
    // Attribute: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_ast/ast/struct.Attribute.html
    // Ident: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_span/symbol/struct.Ident.html
    // Path: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_ast/ast/struct.Path.html
    let smpl_ident = symbol::Ident::from_str("smpl");
    for attr in item.attrs {
        if let ast::AttrKind::Normal(ref attr_item) = attr.kind {
            if attr_item.path.segments[0].ident == smpl_ident {
                println!(
                    "Found: `{}` on item '{}' ({})",
                    pretty_path(attr_item.path.segments.iter().map(|s| &s.ident)),
                    // TODO: Get full path from HirId somehow?
                    item.ident,
                    pretty_macro_args(&attr_item.args),
                );
                return true;
            }
        }
    }

    false
}

fn main() {
    // NOTE: Input program needs to register the tool attribute
    // And you ALSO need to set rustc_session::config::Options.unstable_features properly
    // See:
    //   1) Issue #44690 for RFC 2103
    //          https://github.com/rust-lang/rust/issues/44690#issue-258689168
    //   2) PR #66070 for implementations
    //          https://github.com/rust-lang/rust/pull/66070#issue-336079332)
    //
    let input = "#![feature(register_tool)]\n#![register_tool(smpl)]\nstatic HELLO: &str = \"Hello, world!\"; #[smpl::capture(\"root::main\")]\nfn main() { println!(\"{}\", HELLO); }";

    let input1 = config::Input::Str {
        name: source_map::FileName::Custom("main.rs".to_string()),
        input: input.to_string(),
    };

    let input2 = config::Input::File(std::path::PathBuf::from(
        "/home/q/Documents/savestate/analysis-test/src/main.rs",
    ));

    let input = input2;

    let out = process::Command::new("rustc")
        .arg("--print=sysroot")
        .current_dir(".")
        .output()
        .unwrap();
    let sysroot = str::from_utf8(&out.stdout).unwrap().trim();
    let config = rustc_interface::Config {
        // Command line options
        opts: config::Options {
            maybe_sysroot: Some(path::PathBuf::from(sysroot)),
            unstable_features: rustc_feature::UnstableFeatures::Allow,
            ..config::Options::default()
        },
        // cfg! configuration in addition to the default ones
        crate_cfg: FxHashSet::default(), // FxHashSet<(String, Option<String>)>
        input,
        input_path: None,  // Option<PathBuf>
        output_dir: None,  // Option<PathBuf>
        output_file: None, // Option<PathBuf>
        file_loader: None, // Option<Box<dyn FileLoader + Send + Sync>>
        diagnostic_output: rustc_session::DiagnosticOutput::Default,
        // Set to capture stderr output during compiler execution
        stderr: None,                    // Option<Arc<Mutex<Vec<u8>>>>
        crate_name: None,                // Option<String>
        lint_caps: FxHashMap::default(), // FxHashMap<lint::LintId, lint::Level>
        // This is a callback from the driver that is called when we're registering lints;
        // it is called during plugin registration when we have the LintStore in a non-shared state.
        //
        // Note that if you find a Some here you probably want to call that function in the new
        // function being registered.
        register_lints: None,
        //register_lints: Some(Box::new(register_lints)), // Option<Box<dyn Fn(&Session, &mut LintStore) + Send + Sync>>
        // This is a callback from the driver that is called just after we have populated
        // the list of queries.
        //
        // The second parameter is local providers and the third parameter is external providers.
        override_queries: None, // Option<fn(&Session, &mut ty::query::Providers<'_>, &mut ty::query::Providers<'_>)>
        // Registry of diagnostics codes.
        registry: registry::Registry::new(&rustc_error_codes::DIAGNOSTICS),
        make_codegen_backend: None,
    };

    // run_compiler:  https://doc.rust-lang.org/nightly/nightly-rustc/rustc_interface/interface/fn.run_compiler.html
    rustc_interface::run_compiler(config, |compiler| {
        compiler.enter(|queries| {
            // Parse the program and print the syntax tree.
            // let parse = queries.parse().unwrap().take();
            // println!("{:#?}", parse);
            // Analyze the program and inspect the types of definitions.

            // Queries: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_interface/struct.Queries.html
            // TyCtxt: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_middle/ty/context/struct.TyCtxt.html
            queries
                .global_ctxt()
                .unwrap()
                .take()
                .enter(|tcx: rustc_middle::ty::TyCtxt| {
                    println!("DONE");
                    // Item: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_hir/struct.Item.html
                    for (_, item) in &tcx.hir().krate().items {
                        match item.kind {
                            // https://doc.rust-lang.org/nightly/nightly-rustc/rustc_hir/index.html
                            rustc_hir::ItemKind::Static(_, _, _)
                            | rustc_hir::ItemKind::Fn(_, _, _)
                            | rustc_hir::ItemKind::Struct(_, _) => {
                                if is_smpl_item(item) {
                                    let name = item.ident;
                                    let ty = tcx.type_of(tcx.hir().local_def_id(item.hir_id));
                                    println!("{:?}:\t{:?}", name, ty)
                                }
                            }
                            _ => (),
                        }
                    }
                })
        });
    });
}
