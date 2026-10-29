---
name: yaks
description: Yaks task tracking workflow. Use when a .yaks/ directory exists in the project. Provides commands and guidance for managing filesystem-native tasks stored as markdown files with YAML frontmatter.
metadata:
  yaks-version: "0.0.24"
  yaks-digest: "4774bce54debc01f"
---

# Yaks — Task tracking workflow

This project tracks work with yaks: each task (a **yak**) is a markdown file with YAML frontmatter in a `.yaks/` farm. Follow this workflow so task state stays accurate.

## Running yaks

A plain CLI, run from the shell (no slash commands, nothing to keep running): `yaks <cmd>` if it is on `PATH`, else `npx @j15r/yaks <cmd>`, else `./target/release/yaks <cmd>` in a checkout you built. Add `--json` to any query command for machine-readable output; `yaks create --json` also prints the new id and path.

**Which farm am I on?** `yaks path` (no arguments) prints it. Discovery uses `$YAKS_DIR` if set, else walks up from the cwd to the git top-level looking for `.yaks` (a directory, a symlink, or a pointer file), else uses the repo's **farm branch** worktree (below). It never walks past a git top-level, so a nested checkout does not read another checkout's farm by accident; the error names the fixes.

**Starting a farm.** `yaks init --mode branch|private|pointer [--skills default|coordination]` creates the farm, its git excludes and the project-local skills in one idempotent step (re-run it to add what is missing; it never overwrites an edited skill). `branch` is the recommended mode; `--path <dir> --herd <prefix>` goes with `pointer`.

## Where the farm lives

**On its own branch (recommended; this repo).** The farm is an orphan branch (`yaks`, or `git config yaks.branch`) holding only `.yaks/`, checked out once per machine in a worktree outside every checkout (under `~/.local/share/yaks/farms/`). Every checkout of the repo (the main one, git worktrees, agent clones) reads and writes that same farm, so yak edits are live everywhere and code branches carry no yak files.

- **Never stage `.yaks/` with your code.**
- **`yaks commit`** commits the farm on its branch, from any cwd. **`yaks status`** shows what is uncommitted, and whether the branch is ahead or behind its remote.
- **`yaks sync`** commits, pulls (a rebase whose merge driver turns two machines' edits to one yak into a valid yak) and pushes. Nothing syncs on its own: **sync at session start, after landing work, and before switching machines.** If someone else coordinates, leave the sync to them unless asked. In `yaks tui`, `W` commits and `U` syncs.
- **`yaks private`** keeps the farm on this machine: a `pre-push` hook refuses any push of the farm branch, and `yaks sync` commits and pulls but never pushes. `--off` undoes it.
- A fresh clone needs no setup: the first command creates the farm worktree from `origin/yaks`. `yaks migrate --to-branch` moves a committed in-tree `.yaks/` onto a branch. A checkout that still holds an old committed `.yaks/` gets a warning and is redirected to the branch farm.

**Other layouts.**
- **Private (local-only):** `.yaks/` is excluded from git (`.git/info/exclude`). Never commit yak files, and never put yak ids in commit messages, code or comments.
- **Pointer:** a `.yaks` *file* with `path: <farm>` and optional `herd: <prefix>` points one or more repos at a shared out-of-tree farm (`yaks init --mode pointer`). `yaks merge` consolidates separate farms.
- If in doubt, act as private: a leaked id is worse than a missing one.

> **Footgun:** `git clean -fdx` deletes an excluded `.yaks/`. Back up a private in-tree farm, or use a private branch farm instead.

## Vocabulary

- **States** (adjectives): **hairy** (not started), **shaving** (in progress), **shorn** (done), and the hidden **dead**.
- **Verbs** (the commands): **shave** (hairy → shaving), **shear** / `yaks shorn` (shaving → shorn), **regrow** (shorn → hairy), **slaughter** (→ dead), **revive** (dead → hairy).
- **Farm**: one `.yaks/` store. **Herd**: the yaks sharing one id prefix (a farm can hold several; `yaks create --herd <h>`). **Family**: a parent yak and its descendants. **Shed**: one checkout of the repo that someone works in.

## Hard rules

1. **Never write code without a shaving yak.** Even a one-line fix: `yaks shave <id>` first (create the yak if needed).
2. **Shear as soon as the work is done:** evidence and a summary note, then `yaks shorn <id>`.
3. **Keep yak ids out of external surfaces:** PR titles and bodies and external trackers. Ids go in commit messages only when the farm is published (a shared branch farm); with a private farm (`yaks private`, excluded or pointer) keep them out of commits, code and comments too. Use the external key instead (`yaks rollup --keys`; the **yaks-tracker** skill). `yaks scan-ids [FILE]` exits non-zero if text holds a real yak id; wire it into a hook.

## Workflow

1. **Session start:** on a branch farm, `yaks sync` (unless a yakherd coordinates); then `yaks inbox --for agent` (answers and requests nobody has acted on: handle these first); then `yaks next`.
2. **Before code:** `yaks shave <id>`, and its parent if still hairy. Re-read it with `yaks show <id>`; the latest note is the freshest signal.
3. **While working:** `yaks update <id> --note "..."`, one event per note.
4. **Done:** gather evidence, add a short summary note (see "Evidence before you shear"), `yaks shorn <id>`. Shear the parent when its last child is shorn; never leave a hairy parent over shorn children.

**You share the farm with a human.** They edit yaks in `yaks tui` while you work, so a yak may move, gain a note or appear without you. That is signal, not an error: don't revert, restage or report it; follow it if it redirects your work.

## The inbox: work waiting on someone

A yak's `needs` field says who acts next, and `yaks inbox` lists every yak that has one: **awaiting a human** (`needs: human`) or **awaiting an agent** (`needs: agent`, marked `answered` or `requested`).

**Asking a human.** For a decision only a human should make (an ambiguous requirement, a risky tradeoff, a missing credential), don't guess:

- `yaks ask <id> --note "the question"` sets `needs: human` and drops the yak out of `yaks next`. Then return control.
- The human runs `yaks answer <id> --note "..."` (or `a` in the TUI), which sets `needs: agent`. Never answer your own ask.
- `yaks inbox` marks a yak `replied` when the human answered with a plain note instead of `answer`.
- Also ask to *record* a real design fork (behavior a user sees, a data format, a dependency) even when you have a lean: state the lean in the question.

**Requests.** A human (or a yakherd) hands you work with `yaks request <id> --note "..."` (`a` in the TUI on a yak that isn't awaiting them): a chore on that yak, such as tidying its markdown, summarizing its notes or checking something. Treat it as an instruction from the human, scoped to that yak, and do it now unless it needs a decision (then `ask`). A canned one (`--kind tidy|summarize`) arrives as a `Tidy request:` / `Summarize request:` note carrying its whole instruction: follow it literally. It ends with `yaks pickup <id> --drop-request`, which removes the request note so the yak shows only the result; use that for any request whose result speaks for itself.

**Taking it on.** For either kind, `yaks pickup <id> --note "what I'll do"` is the only thing that clears `needs: agent`; then do the work and note the result. An inbox row you leave stays there for the next agent.

**Waiting.** `yaks wait --for agent --timeout 10m` blocks until something is awaiting an agent, then prints it like `inbox` (exit 0; exit 2 on timeout). If your harness can run a command in the background, run it there after an `ask` to be woken by the answer; otherwise return and check `yaks inbox` next session. `--new` ignores rows already there when it started.

## Notes and descriptions

Bodies are GitHub-flavoured CommonMark. The TUI renders each note as its own document, and a human reads it there, so write for a reader scanning a pane, not a log file.

**Shape.**

- **A blank line between every block.** A single newline is only a space: lines with no blank line between them join into one paragraph. Separate each paragraph, list, heading, code fence and table from its neighbours by a blank line. Don't hard-wrap prose.
- **Short blocks.** A paragraph makes one point in a few sentences. Parallel facts (results, files, per-case numbers, next steps) are a list, one item each. A long body with distinct parts gets `##` headings.
- Fence commands, code and pasted output, with a language (```` ```sh ````). Put ids, paths, flags and symbols in backticks.
- Also rendered: tables, task lists (`- [ ]`), footnotes, strikethrough and alerts (`> [!NOTE]`, `TIP`, `IMPORTANT`, `WARNING`, `CAUTION`). Not rendered: inline HTML (shows as literal text), math, emoji shortcodes; mermaid shows as a code block.
- Keep a `Decision:` note (the choice and the alternatives rejected) separate from progress notes.

Not this (one run-on paragraph, because single newlines join):

```text
G1 landed (d84aa65). Splits families into tokens; fullstory joins 161.
NO FIXTURE yet: the hand-built one didn't reproduce the chain.
Next: G3's question, so it belongs with yak-7af5.
```

This:

```markdown
G1 landed (d84aa65): families are now split into their tokens.

- fullstory: 161 joins
- no fixture yet: the hand-built one didn't reproduce the chain

Next: G3's question, which belongs with yak-7af5.
```

**Links.**

- Another yak: its **full id** (`yak-0af1`, never the bare `0af1`); the TUI makes it a link and `yaks refs <id> [--incoming]` lists the references.
- Repo code, in a yak: a path from the repo root behind one `../`, with GitHub's line fragment: `[fn parse](../src/foo.rs#L12-L40)`; `../blob/<sha>/src/foo.rs#L12` pins it to a commit.
- Attachments: `yaks attach` writes the link (`[name](<id>/<file>)`, `![name](...)` for an image). Attach images and long output rather than pasting them.
- Size an image with a trailing alt-text token: `![shot =50%](<id>/shot.png)`, `=300x200` or `=x120` (px, or `%` of width).
- **A yak from your conversation with the human:** when your harness renders links to local files (Zed, Delta, most IDE chats), link the yak's file by its absolute path, as printed by `yaks path yak-0af1` (`[yak-0af1](/home/me/.local/share/yaks/farms/.../yak-0af1.md)`), so they can open it in one click. A yak's file never moves, so the link stays good. (Hard rule 3 still keeps ids out of PRs and trackers.)

**Writing it.** Any note with a newline, a backtick or a `$` goes on stdin: `yaks update <id> --note - <<'EOF'` … `EOF` (quoted `'EOF'` keeps backticks and `$` literal). Never inside `$( )` or a double-quoted argument, where backticks run as commands. `--note-file` / `--description-file` read a file; the same `-` works for `create --description`, `ask --note` and `answer --note`.

## Evidence before you shear

Shear against evidence, not vibes: "it compiles" or a self-report is not enough.

- **A command proves it:** store it with `yaks update <id> --verify '<cmd>'` and run `yaks verify <id>`, which records PASS/FAIL as a note anyone can re-run. `yaks doctor --strict` flags a shorn yak whose `verify:` did not last pass.
- **A web UI change:** a screenshot of the changed state, attached. `sightmap browser`, when available, can find the component and capture just it.
- **A TUI change:** a text frame of the changed state at least; a rendered screenshot if the TUI library can produce one.
- **Code:** link the code that matters (`[fn parse](../src/foo.rs#L12-L40)`); a short snippet only when it is the point.
- **Logs and long output:** the few lines that show the result, fenced; attach the rest.
- Update every surface that describes a user-facing change (in-app help, `--help`, docs) in the same change.
- **No way to verify it?** `yaks ask` whether to build one rather than shearing on faith.

Skip what makes no sense for the change; never skip it because it is effort.

**The summary note** is short: what changed and, unless that already says it all (a typo fix), the shape of the fix in a sentence or two; the evidence (linked or attached, per the list above), and any yaks spawned. Not a bare "done", and not a design doc: a long write-up goes in an attachment the note links to.

`yaks attach` stores the file beside the yak in `.yaks/<herd>/yaks/<id>/` and links it in the body.

## Labels

Few and broad (`ui`, `docs`, `cli`): reuse an existing label before inventing one (`yaks stats` lists them), at most two or three per yak, one word each (`--labels ui,docs` is two labels). A tracker label (`jira`, `github`) marks yaks that map to an external issue.

## Commands

Every command has `--help`; this is the map. Global flags: `-C <path>` (run as if in `<path>`, like `git -C`), `--as <actor>` on any write (else `$YAKS_ACTOR`, else a Delta thread name, else the git user).

| Command | What it does |
|---|---|
| `create "title"` | New hairy yak: `--type --priority --parent --herd --labels --depends-on --source --verify --description` |
| `list` / `next` / `tangled` / `search` | Query (shared filters, below); `next` = hairy with deps met, `tangled` = blocked on deps |
| `show <id>` | Full details, parent and children |
| `path [<id>...]` | Absolute file path of yaks (by id or filters); no arguments: the farm in use |
| `update <id>...` | Fields, `--add-label`/`--remove-label`, `--note`, `--verify`, `--source` |
| `shave` / `shorn` / `regrow` / `slaughter` / `revive` | State moves; several ids at once; each appends an attributed `moved:` entry. `slaughter --family` takes descendants too |
| `ask` / `answer` / `request` / `pickup` / `inbox` / `wait` | The inbox (above); `inbox --for human\|agent`, `wait --timeout 10m` |
| `verify <id>` | Run the yak's `verify:` (else the config default) and record PASS/FAIL |
| `attach <id> <file>` | Store and link evidence; `rename-attachment` renames one |
| `log` | Notes and moves over a filtered set, oldest first; `--since 2h`, `--by <actor>` |
| `refs <id>` | What it points at; `--incoming` for what points at it |
| `commits <id>` | Git commits naming the yak or touching its file |
| `dep` / `reparent` | Add/remove a dependency; move under `--parent <id>` or `--unparent` |
| `bulk` | One field edit or reparent over a filtered set; dry-run unless `--commit`; refuses without a filter |
| `rename` / `rename-herd` | Rename a yak or a whole herd, rewriting every reference; `--dry-run` first |
| `status` / `commit` / `sync` | The farm's uncommitted changes; commit them; commit, pull and push a branch farm |
| `private [--off]` | Keep a branch farm on this machine |
| `init` / `migrate` / `merge` | Create a farm; upgrade an old schema or `--to-branch`; consolidate another farm |
| `doctor [--strict]` / `preflight` | Farm integrity; pre-landing check |
| `stats` / `rollup [--keys]` | Counts; yaks grouped by external issue |
| `skills install` / `skills status` | Install or check these skills |
| `tui` | The interactive UI (`?` lists its keys) |

Coordinating several agents (`brief`, `land`, `sheds`) is the opt-in **yaks-coordinating** skill (`yaks skills install --with coordination`).

## Filtering

`list`, `next`, `tangled`, `search`, `path`, `log` and `bulk` share filters: `--status`, `--type`, `--priority`, `--label`, `--herd` (each repeatable, OR within a flag), `--search Q`, `--ready`, `--tangled`, `--needs`, `--run-on` (a body whose lines join into one paragraph), `--parent-of <id>` (its descendants); AND across flags. `--all` includes dead yaks.

```sh
yaks list --type bug --priority 1
yaks next --label auth
yaks bulk --parent-of yak-c3d4 --add-label spike --commit
```

**Large farms.** Don't summarize yak files through `ls .yaks/... | xargs`: past a few thousand paths `xargs` splits the list and a total like `xargs wc -l | tail -1` is silently wrong, and an absolute glob fails with "argument list too long". List with `yaks list --json` or `yaks path <filters>` (one path per line); to total, feed one stream to one counter (`yaks path --status shorn | xargs cat | wc -l`, never `xargs wc` or `find -exec wc {} +`, which print a total per batch).

## Task format

Each yak is one file, `.yaks/<herd>/yaks/<id>.md`, at the same path for life (a state move edits `status:` in place):

```markdown
---
id: yak-a1b2              # flat {prefix}-{4hex}; never encodes hierarchy
title: Fix the login crash
status: shaving           # hairy | shaving | shorn | dead
type: bug
priority: 2
created: "2026-02-16T10:00:00Z"
updated: "2026-02-16T10:30:00Z"
parent: yak-c3d4          # optional; the only source of hierarchy
depends_on: [yak-e5f6]
labels: [auth]
source: https://jira.example.com/browse/PROJ-123   # optional external issue
---

Details go here.
```

Don't parse ids for hierarchy (old dotted ids like `yak-a1b2.1` are opaque). `.yaks/config.yaml` sets the default `herd`, `default_type`, `default_priority` and a `verify:` map (label → command, plus `default`); a `herds:` map overrides those per herd, one level deep. A farm on an older schema is refused by every command but `yaks migrate`.
