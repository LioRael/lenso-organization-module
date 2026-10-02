# Contributing

This repository is a first-party Lenso Plugin repository. Contributions must
preserve the Plugin/Capability boundaries, the PostgreSQL ownership model,
and the Trusted Publishing rules in [the release process](docs/release-process.md).
Do not add compatibility code for the retired v0.3.x architecture or make
registry credentials part of a change.

## Choose a contribution path

- **Delta**: use the task-owned Delta checkout directly. A maintainer may use
  **Delta Land Changes** (the `/land` workflow) after review; it is a delivery
  path, not a universal shell command or a permission grant. Do not create a
  nested Worktrunk checkout.
- **Other agents or editors**: work in a fork or task-owned clone and keep the
  change focused. Do not edit another contributor's checkout.
- **Plain Git**: a fork and branch are supported. Open a GitHub Issue with the
  immutable commit SHA, scope, validation performed, and known limitations;
  maintainers review and import the exact revision when it is accepted.

The maintainer owns repository landing. A review, an agent, or write access does
not itself authorize a push to `main`.

## Before requesting review

1. Read `AGENTS.md`, the relevant docs, and the affected Plugin or Capability
   contract.
2. Keep commits small and avoid unrelated formatting or generated changes.
3. Run focused checks for the files changed. Rust behavior normally needs the
   affected package checks; workflow, documentation, skill, and configuration
   changes should use YAML/link/diff/script checks. The candidate CI `check` job
   is the authoritative full native, PostgreSQL, and package proof.
4. Report the base SHA, candidate SHA, checks run, and any limitation. Never
   claim PostgreSQL acceptance without `LENSO_POSTGRES_TEST_URL` and the
   corresponding test command.

A durable non-PR handoff may use `git format-patch --binary <base>..<sha>`.
Include the patch and its immutable source SHA in the Issue; the maintainer
imports it, reviews the resulting tree, and runs the candidate gate.

## Maintainer landing contract

After review and any fixes, the maintainer fetches a fresh `origin/main`, records
the full base SHA, and pushes the final commit once to a unique
`delta/verify/<task>/<attempt>` ref. The CI workflow is intentionally attached
to those candidate refs, not pull requests or ordinary `main` pushes. The
maintainer accepts only the `check` run whose repository, workflow, event, ref,
exact head SHA, attempt, and result match the candidate. Then the maintainer
fetches `main` again and normally fast-forwards the **same verified SHA** only
if the destination is unchanged (or already contains that SHA). A changed
base requires a new review and candidate run; failed candidates are never
promoted.

Delta's `/land` workflow follows this contract. Maintainers using other agents
or plain Git follow the same candidate-first sequence manually. No release is
implied by landing: package publication, tags, releases, deployment, and
Trusted Publishing are separate, explicitly authorized operations.
