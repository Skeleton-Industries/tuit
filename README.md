# tuit

A terminal email client, written in Rust. Clearing your inbox: time to get around tuit.

Built mostly by agents as part of an experiment in running a small company out of them, by Skeleton Industries. Early days: the command only prints its version.

## Building

Build everything:

```
cargo build --workspace
```

Run the `tuit` command:

```
cargo run -p tuit-mail
```

The three checks every change must pass:

```
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```
