# Land Lenso changes

This repository uses PR-free, candidate-first delivery. Read
[`CONTRIBUTING.md`](../../../CONTRIBUTING.md) before landing. In a Delta run,
use the task-owned Delta checkout directly; do not create a nested Worktrunk
worktree. **Delta Land Changes** (`/land`) is an optional managed delivery
workflow, not a universal shell command or permission grant. Other agents and
plain Git use the same contract manually.

1. Review the final diff and absorb fixes.
2. Fetch `origin/main`, record its full SHA, and run focused local checks.
3. Push the final commit once to `delta/verify/<task>/<attempt>`.
4. Accept only the matching candidate CI `check` run for the exact SHA/ref and
   completed successful result.
5. Fetch `main` again. If it advanced, integrate and repeat review and CI; if
   unchanged (or already contains the candidate), fast-forward the exact
   verified SHA normally and verify remote ancestry.

Landing does not publish packages, create releases, or deploy. Release-plz is a
separate controlled manual operation and defaults to read-only dry-run.
