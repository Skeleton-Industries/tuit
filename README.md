# tuit

A terminal email client, written in Rust. Clearing your inbox: time to get around tuit.

Built mostly by agents as part of an experiment in running a small company out of them, by Skeleton Industries. Early days: the command shows a list of made-up messages and nothing else yet.

## Building

Build everything:

```
cargo build --workspace
```

Run `tuit`. It opens a full-screen list of made-up messages built into the program, and needs a real terminal:

```
cargo run -p tuit-mail
```

Keys:

| Key | Does |
| --- | --- |
| `j` or Down | Move down |
| `k` or Up | Move up |
| `g` or Home | Go to the top |
| `G` or End | Go to the bottom |
| PageDown, PageUp | Move by a screenful |
| `q` or Ctrl-C | Quit |

`tuit --version` (or `-V`) prints the version and exits.

Run the speed benchmark:

```
cargo bench -p tuit-mail --bench speed
```

It measures two things: running `tuit --version`, and listing a Maildir of 10,000 made-up messages. For each it prints the median, fastest and slowest time, next to the target.

The three checks every change must pass:

```
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Licence

MIT. See [`LICENSE`](LICENSE).
