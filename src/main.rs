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
extern crate rustc_session;
extern crate rustc_span;

mod lint;

use rustc_errors::registry;
use rustc_hash::{FxHashMap, FxHashSet};
use rustc_lint::{LintId, LintStore};
use rustc_session::{config, Session};
use rustc_span::source_map;

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

// Option<Box<dyn Fn(&Session, &mut LintStore) + Send + Sync>>)

fn main() {
    // NOTE: Input program needs to register the tool attribute
    // See:
    //   1) Issue #44690 for RFC 2103
    //          https://github.com/rust-lang/rust/issues/44690#issue-258689168
    //   2) PR #66070 for implementations
    //          https://github.com/rust-lang/rust/pull/66070#issue-336079332)
    //
    let input = "#![feature(register_tool)]\n#![register_tool(smpl)]\nstatic HELLO: &str = \"Hello, world!\"; #[smpl::capture(\"root::main\")]\nfn main() { println!(\"{}\", HELLO); }";

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
        input: config::Input::Str {
            name: source_map::FileName::Custom("main.rs".to_string()),
            input: input.to_string(),
        },
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
    rustc_interface::run_compiler(config, |compiler| {
        compiler.enter(|queries| {
            // Parse the program and print the syntax tree.
            // let parse = queries.parse().unwrap().take();
            // println!("{:#?}", parse);
            // Analyze the program and inspect the types of definitions.
            queries.global_ctxt().unwrap().take().enter(|tcx| {
                println!("DONE");
                for (_, item) in &tcx.hir().krate().items {
                    match item.kind {
                        rustc_hir::ItemKind::Static(_, _, _) | rustc_hir::ItemKind::Fn(_, _, _) => {
                            let name = item.ident;
                            let ty = tcx.type_of(tcx.hir().local_def_id(item.hir_id));
                            println!("{:?}:\t{:?}", name, ty)
                        }
                        _ => (),
                    }
                }
            })
        });
    });
}
