# Superseded: dialog-calibration "Part 2" runs (harness at `76fb295`)

Three runs (ERR-01, CANCEL-01b, PROC-01) intended as automated dialog handling (D2u).

**The operator clicked the dialogs** (confirmed by the operator): each dialog closed about one second after it appeared, and the harness logged no action of its own.

- **Label correction.** The records say `dialog_driver: D2u`, but these are operator-clicked runs. They are **not** evidence for the automated driver. They agree with the D1 runs: the destination file is pre-allocated at full size under its final name, then removed, and nothing remains.
- **False-crash correction.** Two records show `outcome_class: O5`. That class was assigned because short-lived `explorer.exe` launcher processes exited during the run, which the harness then mistook for a crash. The harness now counts only long-lived `explorer.exe` processes (`dec08cc`).

Valid D2u calibration runs: `ERR-01-20261113T184124-d96f5696`, `CANCEL-01b-20261113T184140-c3d9524c` and `PROC-01-20261113T184152-f0afb16c` (top-level `reports/stage-b1/`).
