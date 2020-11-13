#[smpl::map]
pub struct Foo {}

fn bar() {}

fn qux() {}

// argument "hello maps" stored as a rustc_ast::Token inside a rustc_ast::tokenstream::TokenStream
//   (https://doc.rust-lang.org/nightly/nightly-rustc/rustc_ast/tokenstream/struct.TokenStream.html)
//   inside of rustc_ast::ast::MacArgs::Delimited.2
//
//   This is PROBABLY different from a procmacro's TokenStream
#[smpl::map("hello maps")]
fn hello() {}
