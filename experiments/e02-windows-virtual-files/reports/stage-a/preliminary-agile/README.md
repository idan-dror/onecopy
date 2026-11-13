# Preliminary Stage A runs — agile objects (not variant T1)

These 18 run records were produced at commit `6f1b9e1`, with driver D2c and three repeats of A-01 to A-06. All 18 passed: digests matched, the clipboard was empty after exit, and memory stayed flat.

**They do not satisfy threading variant T1.** windows-rs `#[implement]` makes objects agile by default (it implements `IAgileObject` and a free-threaded `IMarshal`; `windows-implement` 0.60.2). Cross-process calls from Explorer were therefore dispatched on RPC worker threads, not the owner STA. The agent's `start` event still recorded `threading = "T1"`, which is mislabelled for these runs.

`thread-analysis.json` quantifies this from the raw event logs: 2,680 COM-call events arrived off the owner thread, and 18 on it. The 18 on-thread events are the agent's own enumeration during `OleSetClipboard`.

The fix (`Agile = false`, which gives true T1) and the authoritative Stage A runs are in the parent directory. These records are kept as data on agile-object behavior. That behavior is relevant to Stage B variant T2 (calls off the STA), which these runs do not evaluate.
