---
name: yaks-coordinating
description: "Use when you will hand yaks to other agents or land their work: coordinating several agents and a human over one yaks farm (claim, brief, fan out, land, reconcile), in Delta threads or plain git worktrees."
metadata:
  yaks-version: "0.0.27"
  yaks-digest: "369f0c8993e4cdab"
---

# Coordinating yaks

You are the **yakherd**: the one agent that claims yaks, briefs workers, lands their work and talks to the human. A **worker** owns exactly one yak and follows `yaks-working`. One agent with no helpers needs only `yaks-working`.

**The design test.** If something is a query or mutation over the task files, it belongs in the yaks tool; if it assumes subagents, worktrees or an orchestration, it belongs here. With one agent yaks is memory and a log; with many it is a shared blackboard.

## 1. The farm everyone shares

`yaks path` (no arguments) prints the farm: its branch's worktree, one per machine, which every checkout (yours, the human's, each worker's) reads and writes live. So:

- Claims, notes, shears and asks are visible to everyone the moment they are written. There is no yak state to merge and nothing under `.yaks/` in any code commit.
- **You commit the farm** (`yaks commit`), at least at every landing. **You sync it** (`yaks sync`) at session start, after landings and before a machine switch. On a machine nobody is sitting at (a cloud runner, a VM), also sync at every check-in on your workers: the human sees nothing until you do, and a reset of that machine deletes every unsynced write. Until the human has published the farm (`yaks status` shows no upstream), use `yaks sync --no-push`: the first push creates the remote branch.
- **Names are identity.** Every worker writes under its own short `YAKS_ACTOR` (the brief sets it), and so should you: `yaks sheds` groups work by that name, and a name that changes mid-session splits one worker into two.
- Workers find the farm by themselves: from a git worktree or a Delta clone, discovery reads the host repo's farm branch (and its `yaks.farm`, when the repo shares another's farm). No `YAKS_DIR` needed.
- Yak ids are fine in commit messages. On a **private farm** (`yaks private`) keep yak ids out of commits, code, comments and PRs (`yaks scan-ids` checks text; `yaks brief` says so), and land with `yaks land <sha> --yak <id>` so the landed SHA is recorded on the yak: it is the only link from the yak to its code.

## 2. The shape: claim, fan out, land, reconcile

1. **Claim.** Create or pick the leaf yaks, `yaks shave` each, and write a claim note: owner name, scope by file and symbol, out of scope, evidence contract, judge (section 4).
2. **Fan out.** One worker per yak, each from your freshest state, each with a brief (section 5).
3. **Land.** Only you land, one worker at a time, by the SHA in its final message, then check it (section 6).
4. **Reconcile.** A yak still `shaving` after the batch is stalled: `regrow` it or re-run it. `yaks doctor`, `yaks commit`.

**Watching workers.** `yaks log --by <worker> --since 1h` and `yaks inbox` show their notes and asks live. `yaks sheds` lists everyone writing to the farm, one row per actor name, on any machine once synced: what each is shaving, what it touched recently, its open asks, its latest checkpoint branch, and when it last wrote: `active` (wrote in the last 2 hours), `stale` (holds a claim but has written nothing for 2 hours), else `idle`. With heartbeat notes, that tells a slow worker from a dead one.

## 3. Scope work so it cannot collide

- **One writer per yak.** Never edit a yak a worker owns while it runs; summarize onto the parent yourself afterwards. Shared work gets split into child yaks first.
- **Types, not only files.** A change to a shared type (a struct built in many places) breaks the other workers at merge: land it first as your own prep commit. Scan with `grep 'TypeName {'`.
- **One big file?** Give each worker a disjoint function or region, anchor by symbol (`fn name`), never by line number, and warn that a shared render path can ripple into snapshots.

## 4. Evidence contract and judge

The claim note says what counts as done and who decides, and the judge shears: a script (the yak's `verify:` or the config default; the worker shears on PASS and you re-run it), the human for a subjective call (`yaks ask`), or you at landing. If you cannot say what the evidence is, the lever is missing: ask the human about building one before you fan out.

## 5. The brief

`yaks brief <id> --as <name>` prints everything the yak, config and farm determine: the yak and that it is shaving, the `YAKS_ACTOR=<name>` prefix (and `YAKS_DIR` if you pass `--yaks-dir`), the binary, the gate, forbidden moves, how to finish for this farm, how to ask and pick up, the `Decision:` rule and the final-message format. It warns if the yak is not claimed. Add only what a yak cannot know:

```
<output of `yaks brief <id> --as <name>`>
Task: <what and what not>. Scope by symbol: <files, functions>. Out of scope: everything else;
if you must touch another file, `yaks ask` instead.
Evidence beyond the gate: <the test that fails before and passes after>.
Do not read, write or build in any other checkout.
```

Pass `--yaks-dir <farm>` when the worker's checkout cannot find the farm. If the task changes yaks itself, tell the worker to build and use its own binary.

## 6. Landing

1. **Land by SHA:** `yaks land <sha> --dry-run`, then `yaks land <sha>`. It lands one single-parent squash commit (or a cherry-pick when the base moved), checks the result against `git merge-tree`, and never pushes. It records a `kind: landed` note (the landed and worker SHAs) on each yak the commit message names, plus any `--yak <id>`; `yaks commits <id>` lists them. It refuses while your tree has changes the commit does not explain, a SHA whose change a merge dropped, and a commit that stages `.yaks/` beside a branch farm.
2. **Re-run the gate yourself** and reproduce the key scenario with your own build. A claim you cannot reproduce is rejected, not annotated: `git revert` the landing and regrow the yak.
3. **Read the diff** for quality and for every surface the change touches (docs, skills, README, `--help`, in-app help); make fixes as your own follow-up commits.
4. `yaks doctor`, then `yaks commit`. Never write a claim into a yak that you did not observe.

## 7. Asks

- **Your own design questions** go on yaks too: in a design pass, ask each question for the human on the yak it concerns, with options and your lean, and link the asks from chat instead of restating them there. The answers then stay on the yaks for whoever picks them up later.
- A worker that needs a decision runs `yaks ask`, leaves its edits and returns.
- **Scope and mechanics** (reversible, local to the yak's goal: one more file, which test shape): answer it yourself with `yaks answer <id> --note "..."`, saying you are the yakherd, then wake the worker. The answer is the record; the message is only the wake-up.
- **A design fork** (behavior a user sees, a data format, a dependency, a shipped surface, anything a reviewer would want a say in; if unsure, it is a fork): leave the ask open, and tell the human the yak id and your lean (record the lean on the parent yak, never the worker's). Wake the worker once `yaks inbox` shows it answered (`yaks wait --for agent --new --timeout <dur>` blocks until it is, if your harness can run it in the background).
- **Waiting on workers' asks:** `yaks wait --for human --new --timeout <dur>` returns when a worker raises a new ask.
- **Handing a chore to an agent** with no worker running (tidy a body, summarize a yak): `yaks request <id> --kind tidy|summarize` (or `--note "..."` for anything else) queues it in the agent inbox for whichever agent runs next. In bulk: `yaks bulk --run-on --request tidy` previews and `--commit` queues (a yak awaiting a human is skipped).
- At session start and after each batch, run `yaks inbox`. A yak marked `replied` got a plain-note answer: record it with `yaks answer`, quoting the human and saying you read it from their note.

## 8. Attribution and drift

Every note and move carries an actor: `--as <name>` or `YAKS_ACTOR=<name>` on each command (else a Delta thread name, else the git user, which is the human). A move is not evidence; a shorn yak still needs a real note. The human edits yaks while you work: a yak that moved or gained a note is a person beside you, not an error; follow it if it changes your task.

## 9. In Delta

Your thread is the yakherd; each worker is a subthread with its own clone, which starts from a snapshot of your working tree.

- **Spawning.** Start each title with the worker's name (`sheds-1: build the sheds CLI`), the same name the brief sets as `YAKS_ACTOR`: that name stamps the worker's notes and is its row in `yaks sheds`. Record the worker's name, agent id, model and base commit on the umbrella yak or in your landing note, never on the worker's yak. Pass `model` only when the human asked for one.
- **What comes back is not trustworthy.** Delta merges a result against what it last applied to your tree, not your HEAD: expect a pending `MERGE_HEAD`, stray modified files, a worker's unfinished edits, even files reset to an old snapshot. Never `git add -A` after a worker returns and never hand-revert Delta's applied state; `yaks land <sha>` sorts it out. While any worker is running or parked, don't rewrite your branch (no `reset --hard`, no rebase): its old base turns the next landing into add/add conflicts.
- **Waking a worker.** `send_agent_message` to its thread id; it resumes with its context. Tell it to `yaks pickup <id>` when an answer is waiting.
- **A worker can die silently** (most often when the human moves the thread to another machine). Overdue with no new note (its `yaks sheds` row goes `stale`): presume it dead, have a replacement review its draft, and after two deaths finish the task yourself.
- **Landing in the human's checkout.** Thread commits reach the human's repo on no branch; `main` moves only by your `git push local <branch>:main`. First run `yaks preflight --push-main` (on a shared machine `local` can be a bare repo the human never looks at; it then names the fallback). Never push `origin`'s `main` or open a PR without the human's say-so.
- **Workers on another machine than the human** (a cloud runner): one branch per worker. Have each push its own work branch to `origin` at checkpoints (any name the harness likes, but not under `yaks/`, which the farm branch blocks) and record it with a checkpoint note, so the work is visible before it lands (`yaks commits <id> --fetch` lists it). Say so in the brief, which otherwise forbids `git push`.
- **On another machine,** `local` is that machine's checkout and may be behind: `git fetch origin`, check `yaks preflight --push-main`, and fast-forward it with `git push local origin/main:main`. Then `yaks sync`.
- Project skills load only from real files in `.agents/skills/`; the `skill` tool needs the worktree argument. Still put the critical rules in the brief.

## 10. With git worktrees

- **Cut a worktree** for each worker from your freshest commit: `git worktree add wt/<name> -b <branch>`, in a directory that is not gitignored (some harnesses' file tools refuse ignored paths). Stage explicit paths; never `git add .`. `git worktree remove` it afterwards.
- **File tools rooted at the main checkout:** always edit through the explicit worktree path (`wt/<name>/src/foo.rs`); a bare `src/foo.rs` silently edits the main tree. After edits, `git -C <main> status --short` must show no stray source changes.
- **Each worktree builds and tests its own binary.**
- **Serial arc** (phases that edit the same files in order): keep one worktree for the arc, land each phase to main as it completes, then `git merge main` back into it.
- **Recovery:** a lost session is not lost work. `git -C wt/<name> status` and `diff`, build that worktree, then finish it yourself or discard and re-run.
