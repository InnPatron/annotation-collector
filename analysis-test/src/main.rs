#![feature(register_tool)]
#![register_tool(smpl)]

mod internal_mod;

fn main() {
    println!("Hello, world!");
}

#[smpl::map()]
fn test() {}
