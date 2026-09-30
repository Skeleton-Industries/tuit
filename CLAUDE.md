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
- `cargo fmt`, `cargo clippy` and `cargo test` must pass before anything is called done.
