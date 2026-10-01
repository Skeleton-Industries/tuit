# Architecture

_Written on 1 October 2026, before the first line of Rust. It's a direction, not a law. Change it by pull request when the code teaches us something._

## What tuit is for

**Now:** a terminal email client that's usable, familiar and good to look at. The bar is lazygit, superfile and cliamp.

**Later:** one place on your own machine where everything that piles up arrives, starting with email. You manage it, or an LLM you trust does. Today an LLM needs to speak to a hundred services to read your inbox. With tuit it speaks to one small local tool.

There's no service behind it. You bring your own accounts, and your mail and passwords stay on your machine.

## Three ways in, one core

| Way in | Example | Like |
| --- | --- | --- |
| The full-screen app | `tuit` | lazygit |
| One piece, inline in your terminal | `tuit compose` | pop |
| Plain commands for scripts and LLMs | `tuit list --json` | git |

All three are thin. Each one calls the same core, so a feature built for one is nearly free for the others.

```
  full-screen app      inline forms       plain commands
        \                   |                   /
         +------------------+------------------+
         |   core: messages, decisions, rules  |
         +------------------+------------------+
        /                   |                   \
  Maildir on disk     IMAP sync (later)    sending (later)
```

## The one rule

**The edges depend on the core. The core depends on none of them.**

The core knows what a message is and what you can do with one. It doesn't know about files, the network or the terminal. Everything that touches the outside world is an edge: the Maildir reader, the screen, the command line, and later IMAP and sending.

Why it matters:

- **Swapping is cheap.** A different mail store is a new edge. Nothing in the core changes.
- **Testing is cheap.** The core can be tested with made-up messages, with no disk and no network.
- **Risk is easy to find.** Code that handles passwords, talks to the network or deletes mail can only live in an edge, so a reviewer knows where to look.

## How Rust holds us to it

If you know Rails, two ideas carry over.

- **Crates are like gems.** The repo is a *workspace*: several small crates built together. Each crate lists what it depends on in its own `Cargo.toml`. The core's list has no file, network or terminal crates in it, so core code that tried to open a socket wouldn't compile. The rule is checked by the compiler, not by good intentions.
- **Traits are like interfaces.** A trait is a named set of methods, much like duck typing in Ruby, but checked at compile time. The core defines a trait such as `MailStore` ("can list messages, can fetch one"). The Maildir crate implements it. The app is handed "something that is a `MailStore`" and never learns which one.

Crates for the first slice:

| Crate | Job | May depend on |
| --- | --- | --- |
| `tuit-core` | Messages, and the traits the edges implement | Nothing that does I/O |
| `tuit-maildir` | Reads a Maildir folder | `tuit-core` |
| `tuit-tui` | Draws screens and handles keys | `tuit-core` |
| `tuit-mail` | The `tuit` command. Wires the others together | All of the above |

The package is `tuit-mail` because `tuit` is taken on crates.io. The command you type is still `tuit`.

## Decisions so far

| What | Why |
| --- | --- |
| Mail is read from a local Maildir (one file per message). Fetching from the server is a separate job that fills it. | Reading local files is fast and works offline. Other Unix tools can read the same folder. Changing where mail comes from doesn't touch the reader. |
| Screens are built from pieces that don't know whether they fill the terminal or sit inline. | `tuit compose` and the compose screen inside the app are then the same code. Our terminal library, ratatui, can draw either way. |
| Every action is a core function first, and a key press or command second. | The app, the inline forms and the command line can't drift apart. An LLM gets every feature a person has. |
| Passwords live in the desktop keyring, never in a config file or this repo. | No service behind tuit means the machine is the only thing guarding them. |
| Email is the only source we build for. | The core is "a thing that arrived, and a decision about it", so other sources can come later. We don't write code for sources we don't have. |

## Not decided yet

- **Sending, and what an LLM may send.** Working assumption: anything not sent by a person's own key press lands as a draft and waits for approval.
- **Where remembered decisions live.** Probably a small local database.
- **Which accounts beyond IMAP.** IMAP with an app-specific password comes first.
