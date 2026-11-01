---
name: yaks-working
description: "Use when you are taking one yak from hairy to shorn, as a worker handed one by a yakherd or on your own: re-read it, claim it, keep notes, gather evidence, shear, commit. Minimal and harness-agnostic."
metadata:
  yaks-version: "0.0.27"
  yaks-digest: "201d3b8c57f34bb9"
---

# Working a yak

The habits that leave a trail a human or another agent can trust. **If a yakherd (the coordinating agent) briefed you, follow the brief** (`yaks brief` prints the exact commands, gate and finish for this farm); this skill is the reasoning behind it, and the whole procedure when you work alone.

1. **Re-read the yak now** (`yaks show <id>`, latest note first): someone may have moved it, redirected it or answered a question since you last looked. Check that its dependencies are met.
2. **Claim it** (`yaks shave <id>`, and its parent if hairy) unless your brief says it is already shaving. Name yourself on every command (`YAKS_ACTOR=<name>` or `--as <name>`): `yaks sheds` shows who is working on what by that name, so keep it short and the same all session.
3. **Keep a log.** One event per note, in readable markdown: a blank line between blocks, lists for parallel facts (the `yaks` skill's Notes section). Note at each milestone and at least every 30 minutes of work: your latest note is the only sign, on any machine, that you are still alive. A choice a reviewer could argue about gets its own `Decision:` note with the alternatives you rejected: a decision that lives only in the code is invisible.
   - **Be visible from other machines.** Nothing you write reaches another machine until the farm syncs. Briefed by a yakherd: it syncs, you don't, unless the brief says otherwise. Working alone on a machine nobody is sitting at: `yaks sync` after every note or state move. When your code lives where others can't read it (and your brief, if any, allows pushing), commit, push your work branch (never `main`, and not under `yaks/`, which the farm branch blocks), note it (`yaks update <id> --note "..." --meta kind=checkpoint --meta branch=<branch> --meta commit=$(git rev-parse HEAD)`), then sync, in that order.
4. **Ask, don't guess, on a real fork** (behavior a user sees, a data format, a dependency): `yaks ask <id> --note "<question and your lean>"`, leave your edits in place and return (or, if your harness runs background commands, `yaks wait --for agent --timeout <dur>` to be woken by the answer). Never answer your own ask; `yaks pickup <id>` takes on the answer when you resume. A `requested` row in `yaks inbox --for agent` is work the human handed you: `pickup` it the same way.
5. **Touch only your yak.** Changes you didn't make (a yak that moved, a new note) are someone working beside you, not an error. Stopping without finishing: a note saying where you stopped; then, if you are giving it up rather than pausing, `yaks regrow <id>` so the claim doesn't read as live.
6. **Shear against evidence, not "it compiles":** a recorded `yaks verify <id>` PASS, or what you saw attached with `yaks attach`. A user-facing change also updates its in-app help, `--help` and docs. Don't self-shear a subjective call: attach it and ask. No way to verify it? Ask whether to build one.
7. **Finish:** a short summary note (what changed and the shape of the fix, the evidence linked or attached, any yaks spawned; a long write-up goes in an attachment), `yaks shorn <id>`, then one commit of your own files with explicit paths. Never stage `.yaks/`. Put the yak id in the message, unless the farm is private (`yaks private`): then keep yak ids and the word "yaks" out of the commit entirely.

Coordinating several workers is `yaks-coordinating`.
