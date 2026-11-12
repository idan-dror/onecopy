# Phase 0 experiment specifications

**Status:** experiment specifications are **PROPOSED** until the owner approves the documentation baseline and the individual specification. No experiment code may be written before that approval (`CLAUDE.md` §Workflow clarifications, proposed).

| ID | Specification | Status | Implementation |
|---|---|---|---|
| E-02 | [Windows clipboard virtual-file paste](E-02_WINDOWS_VIRTUAL_FILE_PASTE.md) | PROPOSED, implementation-ready | Not started |

Conventions for every specification:

- **Code location.** When approved, experiment code goes in `experiments/<id>-<slug>/`, an isolated workspace that shares no crates with product code.
- **Evidence.** Raw evidence stays in that workspace's git-ignored `evidence/` directory. Only sanitized summaries are committed.
- **Data.** Synthetic data only. Committed artifacts never contain user names, machine names or user-profile paths, because this repository is public.
- **Results.** Every result is recorded in the evidence format of `docs/PHASE_0_VALIDATION.md` and classified as *pass*, *fail*, *inconclusive* or *not run* (with the reason).
