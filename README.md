# tuit

A terminal email client, written in Rust. Clearing your inbox: time to get around tuit.

Built mostly by agents as part of an experiment in running a small company out of them, by Skeleton Industries. Early days: `tuit <maildir>` shows the messages in a Maildir as a list, newest first, and nothing else yet.

## Building

Build everything:

```
cargo build --workspace
```

Run `tuit` on a Maildir, the folder that holds `cur`, `new` and `tmp`. It reads the messages, newest first, and opens a full-screen list of them. It needs a real terminal:

```
cargo run -p tuit-mail -- path/to/maildir
```

`tuit --sample` shows a list of made-up messages built into the program instead. `tuit --help` prints how to use it, and so does `tuit` with no argument. A folder whose name starts with `-` is given as `./-name`.

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

It measures three things: running `tuit --version`, listing a Maildir of 10,000 made-up messages, and handling a key press and redrawing the list with 10,000 messages loaded. For each it prints the median, fastest and slowest time, next to the target.

The three checks every change must pass:

```
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Licence

MIT. See [`LICENSE`](LICENSE).
