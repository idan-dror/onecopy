# Superseded: first D1 dialog-calibration runs (harness at `3835aab`)

Three runs (ERR-01, CANCEL-01b, PROC-01) with the operator clicking the Explorer dialogs (D1). **Kept as raw facts only. Their recorded classification is wrong.**

| Run | Recorded | Correct reading of the raw facts |
|---|---|---|
| `ERR-01-20261113T182754-0630365f` | success / O4 | **failure / O1.** An error box ("Unspecified error") appeared and the operator pressed OK. No destination file remained. |
| `CANCEL-01b-20261113T182808-58fcd8a7` | success / O4 | **cancelled / O1.** The progress window appeared and the operator pressed Cancel. The file was pre-allocated at 64 MiB under its final name, then removed. |
| `PROC-01-20261113T182824-e41b7c81` | success / O4 | **failure / O1.** An error box ("The remote procedure call failed.") appeared and the operator pressed OK. No destination file remained. |

**Harness defects these runs exposed** (fixed in `76fb295` and `dec08cc`):

1. **Labels.** The Windows UI language here is not English, so English button labels never matched. Buttons are now matched by control id.
2. **Attribution.** Explorer's copy error box names neither the file nor the folder, so token-only attribution missed it. The verdict therefore defaulted to "success". Attribution now follows the owner chain.
3. **Missed files.** 100 ms directory polling missed destination files that lived for less than one interval. Change notifications were added.

No run after `76fb295` is affected.
