---
name: babysit
description: Crunch through a todo/backlog list by spawning a fresh sub-agent per item (a babysat "ralph loop"), then verifying each result independently before moving on. Use when the user wants to autonomously work through a checklist/backlog one item at a time, supervising delegated agents rather than doing the work directly.
---

# Babysit

Drive a backlog to completion by delegating one item per sub-agent and verifying
its work yourself before the next. You are the supervisor, not the implementer.

## Loop

For each item, run the sub-agent in the **foreground** (don't poll a backgrounded
PID — it may finish mid-sleep) and tee its log:

```bash
pi --model <model> -p @path/to/backlog.md "Pick the NEXT item from <section>: \
<item>. Follow the established workflow (templates: <recent commits>). \
<concrete scope + gotchas>. Run <project checks>. Update the backlog doc \
(move item to Shipped, terse) + changelog. Commit conventional." \
2>&1 | tee /tmp/ralphN.log | tail -25
```

Then **verify independently** before continuing:

```bash
git log --oneline -1 && git status -s          # committed? clean tree?
<project test cmd> | tail            # tests actually pass
<lint/typecheck/schema checks>
# spot-check the feature end-to-end yourself (don't trust the summary)
```

Repeat until the backlog section is empty.

## Rules

- **One item per agent.** Fresh context each time keeps quality high.
- **Steer the prompt.** Point at recent commits as templates, name the exact
  scope, and pre-warn known gotchas (naming conventions, codegen quirks, the
  right verify command). Update these hints as you learn them.
- **Verify, don't trust.** Re-run tests + the project's `--check` scripts, and
  exercise the feature yourself (e2e render, round-trip, etc.). Summaries lie.
- **Catch process drift.** e.g. agents running a bare test runner that skips a
  freshness gate — rebuild/recheck yourself.
- **Resume on interruption.** If a run dies, `pi -c -p "your previous run was
  interrupted; check git status/diff and finish + commit"` continues it.
- **Keep the backlog doc as the source of truth** — agents read it to pick the
  next item, so keep it terse and current (done → Shipped).

## When done

Run the full check suite once more, do a final end-to-end dogfood, and report a
terse summary of what shipped + any follow-ups noted in the doc.
