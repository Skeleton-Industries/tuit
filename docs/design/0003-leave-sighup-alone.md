# 0003: Leave SIGHUP to its default

**This replaces part of 0001, decision 2.** 0001 said tuit catches SIGHUP as well as SIGTERM. This says it catches only SIGTERM.

## What's being decided, and why now

Building 0001 showed that catching SIGHUP does harm and no good.

- SIGHUP mostly means the terminal window has closed. Then there's no terminal left to put back.
- Once tuit catches SIGHUP, a closed window no longer ends it. The library that reads the terminal, crossterm, loops for ever inside its read once the terminal has gone, so tuit's loop never gets back to see the signal. tuit is left running at full CPU with no window.
- Left to its default, SIGHUP ends tuit at once. That's what happens on `main` today.

## The decision

tuit catches SIGTERM only. On SIGTERM it puts the terminal back and ends by that signal, as 0001 says. SIGHUP is left to the operating system's default, which ends tuit at once.

## Considered and turned down

- **A separate thread to handle both signals.** It does end tuit when the window closes. But it races the screen: a signal while the screen is being set up, or during a draw, can leave the terminal raw. Fixing that needs a lock between the two, which is more code for a case that needs none.
- **Waiting for crossterm to be fixed.** It might never be, and the default does the right thing now.

## What it costs

- A `kill -HUP` sent to tuit while its window is still open leaves that terminal raw, as any `kill` does today. Nothing sends that in practice; `kill` with no option sends SIGTERM.
- A tuit started with SIGHUP ignored, under `nohup` for instance, still spins when its window closes, as it does today. A full-screen program under `nohup` isn't a real case.

## The pull requests it leads to

1. **SIGTERM** (0001, decisions 1 and 2 as amended here). Its tests include closing the window, which must end tuit by SIGHUP. That test fails if anyone catches SIGHUP again.
