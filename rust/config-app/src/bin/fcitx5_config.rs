#![deny(unsafe_op_in_unsafe_fn)]

#[path = "../main.rs"]
mod app;

fn main() {
    app::main();
}
