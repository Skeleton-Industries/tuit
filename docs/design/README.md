# Design documents

One file per design: a decision, or a few that are made together. Each arrives as its own pull request and is reviewed like code: questions and changes on the pull request. Merging it is the decision; a file here on `main` is decided. The code follows in later pull requests, which link back to it.

- **Name:** `NNNN-short-name.md`, numbered in order.
- **Sections:** what's being decided and why now; the decision; what was considered and turned down; what it costs; the pull requests it leads to. Short. A reader should finish it in ten minutes.
- **If it changes something already agreed,** its first line says so.
- **Changing your mind:** a later design document replaces an earlier one, or part of it, and says so at its top. The earlier one isn't rewritten; it gains one line saying "Replaced by NNNN", or which part NNNN replaces.
- **This repository is public, and so are these.** Anything that shouldn't be public is unlikely to be a design decision. Keep it out of here.
