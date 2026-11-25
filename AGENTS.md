# Project guidance

## Milestone closeout and acceptance records

For every milestone in `progress/` (M1–M8), create its acceptance report only after the milestone's required acceptance criteria have been verified. Use `progress/M1-acceptance.md`, `progress/M2-acceptance.md`, and the same naming pattern for later milestones.

Before marking a milestone complete:

1. Compare the milestone's change set with its baseline. Exclude unrelated or pre-existing working-tree changes; identify the baseline and head revisions when available.
2. Verify every acceptance criterion against implementation and evidence. Run relevant automated checks and perform required manual/platform checks. Record exact commands, results, and any manual evidence.
3. Mark the milestone complete only when all required criteria pass. If any criterion is partial, failed, or unverified, keep the milestone open and record blockers in its phase tracker; do not publish a completion report that implies acceptance.
4. Once accepted, add the milestone report and update its phase tracker with the completion status and report link.

Each acceptance report must include:

- Milestone, verification date, baseline/head revisions when available, and overall result.
- A changed-files table listing each added, modified, or deleted file and what it implements or changes. Keep the inventory limited to changes belonging to this milestone.
- A bug-fixes table describing each fixed bug, its symptom or root cause, the fix, and regression-test evidence. State explicitly when no bugs were fixed.
- An acceptance-criteria table with each criterion's result (`pass`, `partial`, `fail`, or `not verified`) and linked or reproducible evidence.
- Tests/build/manual checks with exact commands and outcomes, plus remaining risks, exclusions, and follow-up work.

Treat implementation status and acceptance status separately: code presence or a passing build alone does not prove a milestone is accepted. Never infer completion or test results; report unknowns as `not verified`.
