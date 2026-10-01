# tuit

A terminal email client, written in Rust, that might grow into a general inbox handler. Built mostly by agents as part of Scaffold.

This file is shared house rules. It doesn't say who you are: that comes from your agent file in `.claude/agents/`, started with `claude --agent <name>`.

## Company context

The company lives in `~/Projects/scaffold`, synced by Syncthing and not in git. `BRIEF.md` says what we're doing and why, `LOG.md` says what happened, and `team/` holds each role's brief.

- **Leads** (for example `grace`) read it and write to it, to talk to the board.
- **Everyone else** stays out of it. Your lead gives you what you need in the task.

## Rules

- This repo is public. No secrets, tokens, real email addresses or real mail in any commit, ever. Test fixtures are synthetic.
- Nothing merges to `main` without JB's approval on a GitHub pull request.
- No changes to the machine (packages, global config, anything with sudo) without board approval. Leads propose them in `~/Projects/scaffold/SYSTEM.md`.
- Work in small, reviewable pieces. A PR JB can't read in ten minutes is too big.
- Three checks must pass, from the repo root, before anything is called done:
  - `cargo fmt --all --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`

  Run `cargo fmt --all` first to format. The flags matter. Without `-D warnings`, Clippy prints its warnings and still exits clean. Without `--all-targets`, it skips the tests.
