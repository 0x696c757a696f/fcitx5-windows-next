#![deny(unsafe_op_in_unsafe_fn)]
#![cfg_attr(not(test), windows_subsystem = "windows")]

#[path = "../main.rs"]
mod app;

fn main() {
    app::main();
}
