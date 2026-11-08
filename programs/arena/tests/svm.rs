// LiteSVM tests for the arena program: in-process, no validator, with control
// over the clock so every deadline path is exercised. `anchor build` first
// (they load target/deploy/arena.so), then `cargo test -p arena`.

#[path = "svm/common.rs"]
mod common;

#[path = "svm/admin.rs"]
mod admin;
#[path = "svm/cu.rs"]
mod cu;
#[path = "svm/cup.rs"]
mod cup;
#[path = "svm/parity.rs"]
mod parity;
#[path = "svm/room.rs"]
mod room;
#[path = "svm/season.rs"]
mod season;
