# Architecture

_Written on 1 October 2026, before the first line of Rust. It's a direction, not a law. Change it by pull request when the code teaches us something._

## What tuit is for

**Now:** a terminal email client that's usable, familiar and good to look at. The bar is lazygit, superfile and cliamp.

**Later:** one place on your own machine where everything that piles up arrives, starting with email. You manage it, or an LLM you trust does. Today an LLM needs a separate integration for every service where things pile up. With tuit it speaks to one small local tool.

tuit adds no service of its own. You bring your own accounts. Your passwords go only to your mail provider. Your mail goes there too, and to whatever LLM you choose to point at tuit.

## Three ways in, one core

| Way in | Example | Like |
| --- | --- | --- |
| The full-screen app | `tuit` | lazygit |
| One piece, inline in your terminal | `tuit compose` | pop |
| Plain commands for scripts and LLMs | `tuit list --json` | gh |

All three are thin. The logic is written once, in the core. Each way in adds only its own presentation: a screen and keys, or arguments and output.

```
  full-screen app      inline pieces       plain commands
        \                   |                   /
         +------------------+------------------+
         |           core: messages            |
         |      (later: decisions, rules)      |
         +------------------+------------------+
                            |
                     Maildir on disk  <---  IMAP fetch (later)
```

Fetching from the server is not a way of reading mail. It's a separate job that fills the Maildir. tuit only ever reads mail from disk.

## The one rule

**The edges depend on the core. The core depends on none of them.**

The core knows what a message is and what you can do with one. It doesn't touch files, the network or the terminal. Everything that touches the outside world is an edge: the Maildir reader, the screen, the command line, and later fetching and sending.

Why it matters:

- **Swapping is contained, if we ever need it.** A different mail store would be a new edge. The core's contract with stores will grow as we learn (marking mail as read, for one), but the screens don't change.
- **Testing is cheap.** The core can be tested with made-up messages, with no disk and no network.
- **Risk is easier to find.** Code that handles passwords, talks to the network or deletes files belongs in an edge. The decision about *what* to delete is core logic, and needs just as careful a review.

## How the rule is held

Two parts, and it's worth knowing which is which.

- **The compiler holds the direction.** Each crate lists the crates it may use in its `Cargo.toml`, and it can reach no others except through them. The core's list has none of our edge crates and no terminal or mail-protocol crates. Core code that tried to call them wouldn't compile, and Cargo refuses circular dependencies.
- **Review holds "no I/O in the core". A lint catches the obvious accidents.** Rust's standard library can open files and sockets with no dependency at all, so the compiler alone won't stop that. Any change to the core is reviewed with this rule in mind, and a pull request that touches the core says so. That's a house rule in `CLAUDE.md`. The lint is a tripwire for a shortcut taken in a hurry, not a wall. Clippy, the linter, refuses a short list of the obvious calls in the core crate (`crates/tuit-core/clippy.toml`): reading, writing or deleting a file, listing a folder, opening a socket, running a program, reading an environment variable. It also refuses the print macros there (`println!`, `eprintln!`, `dbg!` and the like), outside tests. The list is short on purpose. An entry is added only when a review finds a breach that got through, so each one has a reason. The lint can be switched off with an `#[allow]` or `#[expect]` line, and it can't see inside the core's dependencies, so either of those or a new dependency in the core is something a reviewer stops on.

If you know Rails:

- **Crates are packaged like gems**, and a workspace is like one repo holding several local gems. The difference that matters here: Ruby code can reach anything that's loaded, but a crate can only use the crates it lists.
- **A trait is an interface**: a named set of methods that a type explicitly signs up to. It isn't duck typing. Having the right methods isn't enough; someone has to write `impl MailStore for Maildir`. The core defines a trait such as `MailStore` ("can list messages"). The Maildir crate implements it. The screen code is handed "something that is a `MailStore`" and never learns which one.

Crates for the first slice:

| Crate | Job | May depend on |
| --- | --- | --- |
| `tuit-core` | Messages, and the traits the edges implement | No other crate of ours. Nothing that does I/O |
| `tuit-maildir` | Reads a Maildir folder | `tuit-core` |
| `tuit-tui` | Draws screens and handles keys | `tuit-core` |
| `tuit-mail` | The `tuit` command: the plain commands, and wiring the others together | All of the above |

The first slice runs on made-up mail in test folders. Real mail arrives with the IMAP fetch, which is the slice after.

### Why a workspace

A workspace is a standard part of Cargo. Prior art: ripgrep, Helix, Alacritty and the terminal mail client meli are all workspaces.

Four crates before any code is more structure than a project this size needs on day one, and a boundary drawn this early may turn out to be in the wrong place. The usual advice is to start with one crate and split when it hurts. We split now for one reason: the direction rule is only checked at a crate boundary. Inside a single crate, modules can depend on each other in both directions and nothing objects. If a boundary proves wrong, a crate folds back into a module by moving a folder, editing the crate lists and changing the import paths. There are no versions or releases between the crates while tuit is installed from the repo. Publishing to crates.io would mean publishing and versioning all four.

## Constraints

tuit has to be fast, secure and maintainable, and each of those has to be shown, not taken on trust. The numbers are starting targets, measured on the build machine. We'll tune them as we learn what good looks like. Where the benchmark runs isn't settled: CI's machines are noisier than the build machine, so their numbers may only be good for spotting a change.

Which of the checks in the table below exist today:

- **Formatting, lints and tests:** run in CI on every pull request, and block a merge. `main` requires them to pass.
- **The vulnerability check:** not yet.
- **The core tripwire and the `unsafe` ban:** in place.
- **The benchmark:** not yet. Once it runs, a number that gets worse will be reported and won't block, while we calibrate.
- **The two pull request sections:** in the template in `.github/`, required by a house rule, and held by review.

The vulnerability check and the benchmark come before the first feature, so the first real code is measured from the start.

| | The constraint | How it's checked | What the design does for it |
| --- | --- | --- | --- |
| **Fast** | First screen in under 100 ms. A 10,000-message folder listed in under 200 ms. A key press redrawn in under 16 ms. | A benchmark on every pull request prints the three numbers. | The core does no I/O, so it can be timed on its own. Loading and drawing are kept apart: drawing uses what's already loaded and never waits on disk or network. No code has tested that rule yet. Fetching is a separate job, so a slow server can't stall the screen. |
| **Secure** | No password is stored by tuit, or written to config, logs or this repo. Network code lives only in named edge crates, and is always encrypted. Mail goes to Trash and is never erased by tuit. Mail is untrusted input: no remote images, nothing in a message is run, and a message may carry instructions aimed at an LLM. No `unsafe` Rust in our crates. Every dependency is checked against the public list of known vulnerabilities. | The compiler rejects `unsafe` written in our crates. Each crate has to opt in to the shared setting, and nothing checks that a new crate does. It doesn't look inside dependencies, or at code that a macro defined in another crate, ours or not, generates inside this one. CI checks dependencies against the public list of known vulnerabilities. What happens when a listed problem has no fix yet isn't decided. Every pull request has a "Risk" section that says "none" or names the passwords, network or deletion it touches. The rest is held by review. | Code that handles passwords, the network or file deletion belongs in an edge, so a reviewer knows where to look first. |
| **Maintainable** | Any pull request can be read in ten minutes. Formatting, lints and tests pass on every one. The core never touches disk, network or terminal. Every new dependency is named and justified. | CI runs formatting, lints and tests. Review holds "no I/O in the core", with a lint as a tripwire, as described above. Every pull request has a "New dependencies" section. The ten-minute rule and that section are held by review. | Small crates with one job each, and one direction of dependency. |

One early measurement. A throwaway program picked out the subject line of 10,000 made-up messages on the build machine. Reading each message whole took about 105 ms when the messages were 7 KB each, and about 390 ms when they were 47 KB. Stopping at the end of the headers took about 130 ms at either size. So the real reader has to stop at the headers, and if it does, the 200 ms target isn't out of reach. The limits of the test: the files were held in memory, so no disk was involved, and nothing was parsed, sorted or drawn.

## Names

The package is `tuit-mail` because `tuit` on crates.io is someone else's library. The command you type is `tuit`.

Two things to know. Our `tuit-*` names are free as of 1 October 2026 but not reserved. And another crate, `tuit-bin`, already installs a command called `tuit`.

We keep `tuit` anyway. `tuit-bin` is a git log viewer with about a hundred downloads. Nothing named `tuit` is in Arch's repositories or the AUR as of 1 October 2026, and an Arch-based system is tuit's first target. If someone installs both through Cargo, Cargo refuses the second one and names the package that owns the command, unless it's told to force it. Outside Cargo nothing warns you. So the clash is rare.

## Decisions so far

| What | Why |
| --- | --- |
| Mail is read from a local Maildir (one file per message). Fetching from the server is a separate job that fills it. | It works offline and needs no round trip to a server. Other Unix tools can read the same folder. Changing how mail is fetched doesn't touch the reader. |
| The full-screen app and the inline pieces share the same widgets. | A compose form is drawn by the same code in both. Each mode has its own small setup and event loop, and an inline piece has a fixed height chosen at the start. Our terminal library, ratatui, supports both. |
| Every action is a core function first, and a key press or command second. | The app, the inline pieces and the plain commands can't drift apart. An LLM gets every reading and sorting feature a person has, where sorting means moving and flagging. Sending and deleting are out of the first iteration: see below. |
| tuit has no setting that holds a password, and none is ever in this repo. It gets each one by running a command you choose, such as your password manager's or the system keyring's. | The mechanism is the same on every platform, and it works without a desktop as long as the command you choose does. The system keyring's usually doesn't. It's how mbsync, msmtp and aerc already do it, so we don't invent a secret store. How well the password is guarded depends on the tool you choose. A plain keyring hands it to any program running as you, including an LLM with a shell. A manager that asks first is better, but only while it's locked, and it can't run unattended. The cost: the config file becomes runnable. Anything that can edit it can run commands as you, so tuit never lets a command or an LLM change that setting. |
| The core models email for now, and calls the thing it holds a **message**. | A message means an email message, which is email's own word for it. It isn't our word for everything tuit might hold one day. When a second source exists, the general word is **item**, and a message is one kind of item. We'll generalise then, and not before. Renaming inside the code is cheap in Rust: the compiler finds every use. One trap: JSON field names are usually generated from the names in the code, so a rename inside can change the output with no error. Tests will pin the output. Names that leave the code aren't cheap: command words, the field names in `--json` output, config keys, and anything written to disk. Those get the care. |

## Not decided yet

- **Sending and deleting, and what an LLM may do.** Both are out of the first iteration, meaning the plain mail client that comes before any triage workflow. tuit reads, moves and flags. Deleting here means moving to Trash, since tuit never erases mail itself; most providers empty Trash after a while, so it's a slow delete and not just another move. The same goes for any folder the provider empties by itself, such as Junk, and for the "trashed" flag, a mark that says a message is to be deleted. Moving and flagging leave those out. When they come back in, the working assumption is that both need a person's approval. How tuit tells a person from a program is unsolved: a program can type a command or drive the app as easily as a person can. We aren't building anything towards that answer now, and we accept that it may mean a rewrite later.
- **Onboarding.** Two jobs: setting up the person, once, and adding an account, which happens many times over the life of an install. The aim is one guided trip. tuit should never stop and tell you to go and do something in your secrets manager, and it should never ask you for the password or store it. A sketch, not yet tried: tuit asks which provider, shows how to make an app-specific password there, then runs your keyring's own "store" command, so that tool asks for the password. tuit then writes the matching lookup command into the config and tests the sign-in, which means it reads the password straight away, as it will on every sign-in. This only works for tools whose store command asks for the password itself, such as `secret-tool`, the Linux keyring's. Some password managers, 1Password and Bitwarden among them, don't, and for those the sketch doesn't work as it stands. A keyring default belongs here, as the first choice offered where a keyring exists. That's a trade: the keyring is the easiest option and, as the passwords row says, the least guarded. It wouldn't work over SSH or from a scheduled job, so choosing your own command stays. One thing to settle first. Onboarding writes the one setting we said no command may change, so it has to know a person is at the keyboard, which is the unsolved question above, and that question is parked. So either onboarding waits for it, or the IMAP fetch ships first with a config file you edit by hand. That choice isn't made. Either way it isn't in the first slice.
- **Large folders.** Showing a message list means opening every file. We haven't measured where that stops being fast enough; the benchmark will say when an index is needed. An index is a cache built from the Maildir. It can be deleted and rebuilt, and where the two differ the Maildir is right. Keeping it fresh when another program changes the folder is the hard part, and isn't designed. Searching would be a trait in the core, so a search that asks the server could be another edge later, with the local index as the fallback. That would be the first exception to reading mail only from disk. Keeping the local copy in step with the server is a separate and bigger problem. It arrives with the IMAP fetch, and syncing gets its own design then.
- **Where remembered decisions live.** Probably a small local database, possibly the same one as the index. The catch: a local store isn't shared between your laptop and your desktop. Mail is, because each machine fetches from the same server. See "When a second source arrives".
- **Which accounts.** IMAP with an app-specific password comes first. That covers iCloud, personal Gmail accounts with 2-Step Verification, and Fastmail plans that include IMAP. Work Google accounts usually don't allow it. Microsoft accounts need a different sign-in (OAuth) and aren't planned. The second account is where we first meet more than one source: see the next section.
- **A stricter core.** Rust can build a crate without the standard library (`no_std`), keeping only the parts that need no operating system. That makes "no I/O in the core" a compiler check for the core's own code, with no list at all. It can still be switched back on with one line, and it still can't see inside dependencies. The cost is real: no `HashMap`, paths or clock, and a narrower choice of crates for the core. It's also unusual. `no_std` is normal for code that runs on small devices, and for libraries that want to be usable there, not as a fence around an application's own logic. So it isn't planned. If breaches keep getting past review, a written trade-off comes first.

## When a second source arrives

Nothing here is built or decided. It's a list of what we already know will be hard, kept in one place so it isn't rediscovered.

- **The second mail account is the rehearsal.** iCloud comes first and Gmail second, and Gmail doesn't behave like iCloud: its labels don't map cleanly onto folders. When Gmail goes in, we write down everything in the core that had to change. That list is the best evidence we'll get of what a second kind of source will cost.
- **Remembered decisions on two machines.** tuit's own data isn't shared the way mail is. Three options so far. Carry what belongs to one message in the mailbox itself, as flags or folders, so the mail server shares it; this depends on what each server allows, which we haven't checked. Keep rules, such as "always defer this sender", in a plain text file that you sync as you would any other settings file. Or accept one machine. Whether rules could live in the mailbox too, as a message kept in a folder of its own, we haven't looked into. A source that can't carry any state of ours rules out the first option for that source.
