# tuit

A terminal email client, written in Rust, that might grow into a general inbox handler. Built mostly by agents as part of Scaffold.

This file is shared house rules. It doesn't say who you are: that comes from your agent file in `.claude/agents/`, started with `claude --agent <name>`.

## Rules

- This repo is public. No secrets, tokens, real email addresses or real mail in any commit, ever. Test fixtures are synthetic.
- Nothing merges to `main` without JB's approval on a GitHub pull request.
- No changes to the machine (packages, global config, anything with sudo) without board approval. Leads propose them to the board.
- Work in small, reviewable pieces. A PR JB can't read in ten minutes is too big.
- Once the repo has a `Cargo.toml`, three checks must pass before anything is called done. Run them from the root of your own checkout: your worktree, if you're in one.
  - `cargo fmt --all --check`
  - `cargo clippy --workspace --all-targets -- -D warnings`
  - `cargo test --workspace`

  Run `cargo fmt --all` first to format. The flags matter. Without `-D warnings`, Clippy prints its warnings and still exits clean. Without `--all-targets`, it doesn't lint test code.

## Style

We adopt common guides and don't write our own.

- Formatting: `rustfmt` with its defaults. No overrides.
- Lints: Clippy's default set, with warnings treated as errors. One addition: in `tuit-core`, Clippy also refuses the print macros and a short list of I/O calls, as a tripwire. See `docs/ARCHITECTURE.md`.
- Naming and the shape of public APIs: the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/).
- Anything those don't cover is decided once, in a PR to this file.

## Commits

- Small and atomic: one change per commit.
- Subject line only, under 80 characters, in the imperative: "Add the Maildir reader", "Fix the date parsing".
- A commit that needs a body is two commits. The "why" goes in the pull request description.
- The one line allowed under the subject is the `Co-Authored-By` trailer. It stays, so it's plain which commits agents wrote.
- History is kept as it happened. No squashing, no force-pushing, and no going back to fix old commits when a convention changes. There is one exception, below.
- If a secret, a token, a real email address or real mail is found in a commit, stop and tell your lead. A lead tells JB. Don't push it, and don't try to fix it yourself. This is the one case where history gets rewritten, whenever the leak is found and however old the commit.
- Pull requests are merged on GitHub with a merge commit. GitHub writes that commit, so the rules above don't apply to it. `git log --first-parent --oneline main` then shows each merged pull request as one line.

## Review

- Before a pull request opens, a reviewing agent reads the change, with the job of finding what's wrong. The reviewer is neither the author nor the lead who opens the pull request. The lead arranges this.
- It covers docs and the pull request description as well as code.
- A pull request's description has the sections in `.github/pull_request_template.md`. "Risk" and "New dependencies" are never left out: they say "None" when there is none.
- A pull request that touches `crates/tuit-core` says so in its description, and its reviewers check the change for I/O: files, the network, running programs, the environment, the terminal. Review holds that rule. The lint there only catches the obvious.
- What reaches JB is the version the lead will stand behind. If JB finds dead code, circular logic or a false claim, the review failed.
