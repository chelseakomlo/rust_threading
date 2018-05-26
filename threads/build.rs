// https://github.com/alexcrichton/cc-rs

extern crate cc;

fn main() {
    cc::Build::new().file("src/tasks.c").compile("tasks");
}
