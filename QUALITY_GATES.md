# Deterministic Engineering Quality Contract

Required evidence: actual check command output, new regression tests for behavioral changes, and no deliberate test skips or coverage/CI weakening. Do not mark a task complete when mandatory checks fail. Changes to this document or CI need human review.

**Machine-enforced:** patch whitespace integrity; native settings, updater/migration regression suite and release-note validation.

The `quality` check must be added to required branch status checks to prevent bypass. Existing workflows are retained and may cover additional validation. Any new coverage or performance thresholds must be based on repeatable baselines, not invented numbers.
