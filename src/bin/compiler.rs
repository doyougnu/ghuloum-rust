use ghuloum_rust::infra::varena;
use ghuloum_rust::parser;

// use libc::{mmap, munmap, MAP_ANONYMOUS, MAP_FAILED, MAP_PRIVATE, PROT_READ, PROT_WRITE};
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

    // TODO: can we alloc not in the arena struct
    // let gb_to_reserve = 12 * 1024 * 1024 * 1024; // 12 gigs
    // let base = mmap(
    // ptr::null_mut(),
    // gb_to_reserve,
    // PROT_READ | PROT_WRITE,      // we get read write permissions
    // MAP_PRIVATE | MAP_ANONYMOUS, // PRIVATE + ANONYMOUS to reserve
    // the space then only get page
    // faults on a write (so we reserve
    // memory but don't write until
    // needed)
    // -1,
    // 0,
    // );
    // let allocator = Arena::<Expr>::new();
    let expr = parser::parse(&contents).unwrap();

    println!("The program:\n{expr}");
    println!("The AST:\n{:?}", expr);
}
