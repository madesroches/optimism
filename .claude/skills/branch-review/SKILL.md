---
name: branch-review
description: Code review the current branch or recent commits with verified issues only
argument-hint: "[base-branch or commit range, e.g. HEAD~5]"
allowed-tools: Bash(git *), Bash(clip.exe *)
---

# Code Review — Verified Issues Only

Review code changes with verified issues only.

**Scope:** Determined by `$ARGUMENTS`:
- If a branch name is given (e.g. `main`), review `<base>..HEAD`
- If a commit range is given (e.g. `HEAD~5`), review that range
- If empty and on `main`, review all uncommitted changes (staged + unstaged)
- If empty and on a feature branch, review against `main`

## Process

### Phase 1: Gather context

Determine the review scope per the rules above, then:

**If reviewing a branch or commit range:**
1. `git log --oneline <base>..HEAD` — list commits
2. `git diff <base>..HEAD --stat` — file summary
3. `git diff <base>..HEAD` — full diff

**If reviewing uncommitted changes on main:**
1. `git status` — overview of changes
2. `git diff --stat` — unstaged file summary
3. `git diff` — unstaged diff
4. `git diff --cached --stat` — staged file summary
5. `git diff --cached` — staged diff

### Phase 2: Identify candidate issues

Scan the diff for potential problems:
- Bugs, logic errors, race conditions
- Duplicated patterns that should be extracted
- Missing error handling at system boundaries
- Type safety holes (unsafe casts, `as` assertions)
- Stale dependencies in hooks/effects
- Security concerns (injection, XSS, secrets)
- API design issues (confusing interfaces, leaky abstractions)
- Performance implications (unnecessary re-renders, N+1 queries, missing memoization)
- Test coverage gaps (new code without tests, existing tests invalidated by changes)
- Project convention violations (naming, patterns, style inconsistent with surrounding code)

For each candidate, write a one-line summary and note which files/lines are involved.

### Phase 3: Verify candidates (parallel)

Launch verification agents in parallel using the Task tool with `subagent_type: "Explore"`.

**Grouping strategy:**
- Group candidates that share the same file(s) into a single agent
- Each agent handles one group of related candidates
- If there are 3 or fewer total candidates, verify them all in a single agent instead of parallelizing

**Each agent prompt must include:**
1. The candidate issue(s) to verify — summary, file paths, and line numbers
2. The relevant section of the diff for context
3. The verification checklist:
   - Can the problematic state actually be reached? Trace callers and data flow.
   - Does the UI or type system prevent the scenario? Check component props, select options, type constraints.
   - Is there existing handling elsewhere that covers this case?
   - Is the "missing" code actually unnecessary given the guarantees of the framework or surrounding code?
4. Instructions to return a verdict for each candidate: **confirmed** or **false positive**, with a one-line explanation

Launch all agents in a single message so they run concurrently. Collect all results before proceeding.

### Phase 4: Report

Output a concise list of **confirmed issues only**. For each:
- One-line summary
- File and line reference
- Why it's a real problem (what you verified)

At the end, note how many candidates were dismissed as false positives (no need to list them individually unless the user asks).

### Phase 5: Clipboard

Copy the confirmed issues list to clipboard using `clip.exe`. Keep it terse — one line per issue, no markdown formatting in the clipboard version.
