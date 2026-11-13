# Superseded D2c regression runs at `897004c`

These 9 D2c runs (A-01..A-06, plus A-01/A-04/A-05 again) all passed. They are **superseded** and are not used as calibration or acceptance evidence, for two reasons:

1. **Ownership bug.** They ran on the agent before `9964dc9`. That agent's clipboard-ownership gate was defective: it treated `S_FALSE` from `OleIsCurrentClipboard` as success. The paste observations are unaffected. The exit-ownership fields are not reliable.
2. **Dirty tree flag.** `experiment_tree_dirty` is true because the analysis-only script `scripts/compare-drivers.mjs` was untracked at the time. That script does not affect run behavior.

The authoritative regression is the 9 runs at `9964dc9` in the parent directory.
