//! The core of tuit: what a message is and what you can do with one, plus the
//! traits that the edge crates implement.
//! It must not depend on any other crate of ours, and must not touch files, the
//! network, the process, the environment or the terminal.

#![deny(clippy::print_stdout, clippy::print_stderr, clippy::dbg_macro)]
