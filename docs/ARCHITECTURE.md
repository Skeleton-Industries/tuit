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

Two checks, and it's worth knowing which is which.

- **The compiler holds the direction.** Each crate lists the crates it may use in its `Cargo.toml`, and it can reach no others except through them. The core's list has none of our edge crates and no terminal or mail-protocol crates. Core code that tried to call them wouldn't compile, and Cargo refuses circular dependencies.
- **A lint and review hold "no I/O in the core".** Rust's standard library can open files and sockets with no dependency at all, so the compiler alone won't stop that. Clippy, the linter, will be configured for the core crate with a list of the standard library's file, network and process types and functions, and run with warnings treated as errors on every pull request. It has three limits. It only knows what's on the list. It can be switched off with an `#[allow]` line. And it can't see inside the core's dependencies. Review covers those, so an `#[allow]` or a new dependency in the core is something a reviewer stops on.

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

## Names

The package is `tuit-mail` because `tuit` on crates.io is someone else's library. The command you type is `tuit`.

Two things to know. Another crate, `tuit-bin`, already installs a command called `tuit`, so the two would clash on a machine that installed both. And our `tuit-*` names are free as of 1 October 2026 but not reserved. Whether to live with the clash is an open question below.

## Decisions so far

| What | Why |
| --- | --- |
| Mail is read from a local Maildir (one file per message). Fetching from the server is a separate job that fills it. | It works offline and needs no round trip to a server. Other Unix tools can read the same folder. Changing how mail is fetched doesn't touch the reader. |
| The full-screen app and the inline pieces share the same widgets. | A compose form is drawn by the same code in both. Each mode has its own small setup and event loop, and an inline piece has a fixed height chosen at the start. Our terminal library, ratatui, supports both. |
| Every action is a core function first, and a key press or command second. | The app, the inline pieces and the plain commands can't drift apart. An LLM gets every reading and sorting feature a person has, where sorting means moving and flagging. Sending and deleting are gated: see below. |
| tuit has no setting that holds a password, and none is ever in this repo. It gets each one by running a command you choose, such as your password manager's or the system keyring's. | The mechanism is the same on every platform, and it works without a desktop as long as the command you choose does. The system keyring's usually doesn't. It's how mbsync, msmtp and aerc already do it, so we don't invent a secret store. How well the password is guarded depends on the tool you choose. A plain keyring hands it to any program running as you, including an LLM with a shell. A manager that asks first is better, but only while it's locked, and it can't run unattended. The cost: the config file becomes runnable. Anything that can edit it can run commands as you, so tuit never lets a command or an LLM change that setting. |
| The core models email for now, and calls the thing it holds a **message**. | A message means an email message, which is email's own word for it. It isn't our word for everything tuit might hold one day. When a second source exists, the general word is **item**, and a message is one kind of item. We'll generalise then, and not before. Renaming inside the code is cheap in Rust: the compiler finds every use. Names that leave the code aren't cheap: command words, the field names in `--json` output, config keys, and anything written to disk. Those get the care. |

## Not decided yet

- **Sending and deleting, and what an LLM may do.** Working assumption: both need a person's approval. How tuit tells a person from a program is unsolved. A program can type a command or drive the app as easily as a person can.
- **A keyring default.** Looking the password up in the system keyring with no setup would be friendlier than writing a command. It wouldn't work over SSH or from a scheduled job, so the command stays. Worth adding once the command route works.
- **Large folders.** Showing a message list means opening every file. That's fine for hundreds of messages and too slow for tens of thousands, so an index will be needed.
- **Where remembered decisions live.** Probably a small local database, possibly the same one as the index.
- **Which accounts.** IMAP with an app-specific password comes first. That covers iCloud, personal Gmail accounts with 2-Step Verification, and Fastmail plans that include IMAP. Work Google accounts usually don't allow it. Microsoft accounts need a different sign-in (OAuth) and aren't planned.
- **A stricter core.** Rust can build a crate without the standard library (`no_std`), keeping only the parts that need no operating system. That makes "no I/O in the core" a compiler check for the core's own code, with no list to maintain. It can still be switched back on with one line, and it still can't see inside dependencies. The cost is real: no `HashMap`, paths or clock, and a narrower choice of crates for the core. Worth trying if the lint proves leaky.
- **The command name.** Keep `tuit` despite the clash with `tuit-bin`, or change it.
