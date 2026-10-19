# 0001: Signals, suspend and dates from the future

**This moves "draw the screen first", agreed on 19 October, out of this design and into the index's design (0002).** The reason is under "Drawing first waits for the index".

## What's being decided, and why now

Agreed on 19 October, and decided here:

- a kill signal puts the terminal back;
- Ctrl-Z suspends `tuit` as it does other terminal programs;
- a message dated more than a day ahead shows no date and sorts last;
- if some messages can't be read, the rest are shown with a count.

One of these is a fault today: `kill` leaves the shell in raw mode until `reset`. None of them depends on drawing the screen first.

## Drawing first waits for the index

Drawing first shows the wait for a large folder without shortening it, and it does nothing for an inbox of ten. The archive it's meant for holds 70,000 to 120,000 messages in one folder, and only an index shortens that wait. An index would probably show the list it already has at once, and bring it up to date behind the screen. So the loading screen drawing first needs might never be wanted. That's a judgement, not a finding, and the index's design is where to settle it.

## The decision

**1. One event loop, with no extra threads for input.** The loop waits for a key with a timeout, and checks for signals each time it wakes. A thread that sat reading the terminal couldn't be stopped, and would steal keys from an editor once compose opens one.

**2. Signals.** On SIGTERM or SIGHUP, `tuit` puts the terminal back, then ends by that signal, as a program that didn't catch it would. Those are the two a terminal program gets from outside: a `kill`, or its window closing. Ctrl-C and Ctrl-\ are keys in raw mode, not signals, and `kill -9` can't be caught by anything.

**3. Ctrl-Z.** In raw mode Ctrl-Z arrives as a key. `tuit` puts the terminal back, then stops itself. When it's continued, it takes the terminal again and redraws everything.

**4. Dates from the future, decided in one place.** A store reports the date its message gives, unchanged. The core's sorting step is given "now" by the binary, which reads the clock once. A date more than a day after "now" becomes no date and sorts last. The sample, which today skips that step, will be made to go through it. So the rule is written once and applies to every store, and a store's answer doesn't change with the day it's read. The core still reads no clock.

**5. Unreadable messages.** A store's list returns the messages it could read and a count of the files it couldn't. The screen shows the count. One unreadable file no longer fails the whole listing. A store that can't be read at all still fails before the screen opens, as today.

## Considered and turned down

- **Three threads feeding one channel** (terminal, signals, a loader). It avoids waking on a timer, but the terminal thread can't be stopped (see 1).
- **crossterm's async event stream.** It needs an async runtime: more machinery.
- **Catching SIGINT and SIGQUIT too.** In raw mode they only come from someone typing `kill -INT`. Two fewer cases.
- **A second signal ending a stuck `tuit` at once.** It would leave the terminal raw, which is the fault being fixed, and a stuck `tuit` is a bug to fix, not a case to handle.
- **The future-date rule in the Maildir reader.** Every store would have to repeat it, and a listing would change from day to day.
- **The arrival time in place of a bad `Date:`.** The date column would then mean two things without saying so.

## What it costs

- A signal is noticed up to a quarter of a second late, and the loop wakes a few times a second when idle.
- A message more than a day ahead, from a sender whose clock is badly wrong, shows no date. That's the trade against spam.
- The core's sort takes one more input, "now". The store's list returns one more number.
- `signal-hook`, already built into `tuit` through crossterm, becomes a named dependency. No new crate.

## The pull requests it leads to

Each is readable in ten minutes and leaves `main` working.

1. **Dates from the future** (4). The screen's support for years before 0, which no message can have, goes with it.
2. **The loop and signals** (1, 2). The pseudo-terminal test gains a `kill`.
3. **Ctrl-Z** (3).
4. **Unreadable messages** (5). Touches the core's store trait.
