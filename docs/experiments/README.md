# Phase 0 experiment specifications

**Status:** an experiment may be implemented only after the owner approves its specification (`CLAUDE.md` §Workflow clarifications).

| ID | Specification | Status | Implementation |
|---|---|---|---|
| E-02 | [Windows clipboard virtual-file paste](E-02_WINDOWS_VIRTUAL_FILE_PASTE.md) | **APPROVED** 2026-11-12 (Stage A, then Stage B after review; SEC-04 and DEST-01 to DEST-04 blocked) | Stage A next; separate PR, not merged without owner approval |

Conventions for every specification:

- **Code location.** Experiment code goes in `experiments/<id>-<slug>/`, an isolated workspace that shares no crates with product code.
- **Evidence.** Raw evidence stays in that workspace's git-ignored `evidence/` directory. Only sanitized summaries are committed.
- **Data.** Synthetic data only. Committed artifacts never contain user names, machine names or user-profile paths, because this repository is public.
- **Results.** Every result is recorded in the evidence format of `docs/PHASE_0_VALIDATION.md` and classified as *pass*, *fail*, *inconclusive* or *not run* (with the reason).
- **Input automation.** Keystroke or input automation is permitted only inside approved experiments (D-041).
