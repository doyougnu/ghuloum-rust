use ghuloum_rust::domain::expr_typed_arenas::CompilerContext;
use ghuloum_rust::expr_parser;
use ghuloum_rust::typed_arena_parser;

use std::env;
use std::fs::File;
use std::io::Read;

fn main() {
    let args: Vec<String> = env::args().collect();

    let file_path = &args[1];
    println!("Reading file {file_path}");

    let mut contents = String::new();
    let mut file = File::open(file_path).unwrap(); // panic for now
    let _ = file.read_to_string(&mut contents);

    let gb_to_reserve = 12 * 1024 * 1024 * 1024; // 12 gigs

    let ctx = CompilerContext::initialize(gb_to_reserve);
    let expr = parser::parse(ctx, &contents).unwrap();

    println!("The program:\n{expr}");
    println!("The AST:\n{:?}", expr);
}
