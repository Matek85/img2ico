//! The shared core of img2ico: everything that works on bytes and pixels and
//! needs neither files nor threads, so it runs the same in the command-line
//! program and in the browser (compiled to WebAssembly).
//!
//! The command line (the `img2ico` package at the repository root) and the
//! web version (`crates/wasm`) are thin shells around this crate. Code moves
//! here from the command line step by step, whenever it is free of file
//! access and operating-system threads.

pub mod archive;
pub mod chroma_key;
pub mod convert;
pub mod diag;
pub mod favicon;
pub mod icns;
pub mod icon;
pub mod layout;
pub mod par;
pub mod resize;
pub mod source;
pub mod validate;
pub mod vector;
