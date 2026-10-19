# 0001: Signals, suspend and unreadable messages

Decision 2 is replaced in part by 0003: SIGHUP is left to its default.

**This moves "draw the screen first", agreed on 19 October, out of this design and into the index's design (0002).** The reason is under "Drawing first waits for the index".

## What's being decided, and why now

Agreed on 19 October, and decided here:

- a kill signal puts the terminal back;
- Ctrl-Z suspends `tuit` as it does other terminal programs;
- if some messages can't be read, the rest are shown with a count.

One of these is a fault today: `kill` leaves the shell in raw mode until `reset`. None of them depends on drawing the screen first.

## Drawing first waits for the index

Drawing first shows the wait for a large folder without shortening it, and it does nothing for an inbox of ten. The archive it's meant for holds 70,000 to 120,000 messages in one folder, and only an index shortens that wait. An index would probably show the list it already has at once, and bring it up to date behind the screen. So the loading screen drawing first needs might never be wanted. That's a judgement, not a finding, and the index's design is where to settle it.

## The decision

**1. One event loop, with no extra threads for input.** The loop waits for a key with a timeout, and checks for signals each time it wakes. A thread that sat reading the terminal couldn't be stopped, and would steal keys from an editor once compose opens one.

**2. Signals.** On SIGTERM or SIGHUP, `tuit` puts the terminal back, then ends by that signal, as a program that didn't catch it would. Those are the two a terminal program gets from outside: a `kill`, or its window closing. Ctrl-C and Ctrl-\ are keys in raw mode, not signals, and `kill -9` can't be caught by anything.

**3. Ctrl-Z.** In raw mode Ctrl-Z arrives as a key. `tuit` puts the terminal back, then stops itself. When it's continued, it takes the terminal again and redraws everything.

**4. Unreadable messages.** A store's list returns the messages it could read and a count of the files it couldn't. The screen shows the count. One unreadable file no longer fails the whole listing. A store that can't be read at all still fails before the screen opens, as today.

## Considered and turned down

- **Three threads feeding one channel** (terminal, signals, a loader). It avoids waking on a timer, but the terminal thread can't be stopped (see 1).
- **crossterm's async event stream.** It needs an async runtime: more machinery.
- **Catching SIGINT and SIGQUIT too.** In raw mode they only come from someone typing `kill -INT`. Two fewer cases.
- **A second signal ending a stuck `tuit` at once.** It would leave the terminal raw, which is the fault being fixed, and a stuck `tuit` is a bug to fix, not a case to handle.
- **Treating messages dated in the future as suspect.** That's a heuristic, not a rule, and belongs with the heuristics that flag and sort mail, when they're designed. Tabled until then.

## What it costs

- A signal is noticed up to a quarter of a second late, and the loop wakes a few times a second when idle.
- The store's list returns one more number.
- `signal-hook`, already built into `tuit` through crossterm, becomes a named dependency. No new crate.

## The pull requests it leads to

Each is readable in ten minutes and leaves `main` working.

1. **The loop and signals** (1, 2). The pseudo-terminal test gains a `kill`.
2. **Ctrl-Z** (3).
3. **Unreadable messages** (4). Touches the core's store trait.
