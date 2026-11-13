// Stage B1 timing analysis on the runner's clock (one clock, 200 ms polling resolution).
// Usage: node scripts/b1-timings.mjs <reports/stage-b1>
import { readFileSync, readdirSync, writeFileSync, statSync } from 'node:fs';
import { join } from 'node:path';

const dir = process.argv[2] ?? 'reports/stage-b1';
const read = (p) => JSON.parse(readFileSync(p, 'utf8').replace(/^﻿/, ''));
let corrections = {};
try { corrections = read(join(dir, 'corrections.json')).corrections ?? {}; } catch { corrections = {}; }
const recs = readdirSync(dir)
  .filter((f) => f.endsWith('.json') && !f.startsWith('stage-b1-') && f !== 'corrections.json' && statSync(join(dir, f)).isFile())
  .map((f) => read(join(dir, f)))
  .filter((r) => r.stage === 'B1' && r.result === 'completed' && !corrections[r.run_id]?.result);

const first = (r, pred) => r.timeline?.find(pred)?.t_ms ?? null;
const rows = [];
for (const r of recs) {
  const t = (src, ev) => first(r, (x) => x.source === src && x.event === ev);
  const paste = first(r, (x) => x.event === 'paste');
  const errorEv = first(r, (x) => x.source === 'agent' && ['error_returned', 'read_timeout', 'self_kill'].includes(x.event));
  const kill = t('harness', 'kill_agent');
  const dialogAppears = (r.timeline ?? []).filter((x) => x.source === 'dialog' && x.event === 'appear').map((x) => x.t_ms);
  const dialogGone = (r.timeline ?? []).filter((x) => x.source === 'dialog' && x.event === 'gone').map((x) => x.t_ms);
  const cancel = (r.dialog_actions ?? []).find((a) => a.action === 'cancel' && a.dialog_progress && a.result?.invoked);
  const stallBegin = t('agent', 'stall_begin');
  const stallEnd = t('agent', 'stall_end') ?? t('agent', 'read_timeout');
  const after = (arr, t0) => (t0 == null ? null : arr.find((x) => x >= t0) ?? null);
  const lastGone = dialogGone.length ? Math.max(...dialogGone) : null;
  rows.push({
    case_id: r.case_id,
    run_id: r.run_id,
    threading: r.threading,
    paste_to_first_dialog_ms: paste != null && dialogAppears.length ? dialogAppears[0] - paste : null,
    error_to_error_dialog_ms: errorEv != null ? (after(dialogAppears, errorEv) ?? null) - errorEv : null,
    kill_to_error_dialog_ms: kill != null && after(dialogAppears, kill) != null ? after(dialogAppears, kill) - kill : null,
    cancel_to_dialog_gone_ms: cancel && lastGone != null ? lastGone - cancel.t_ms : null,
    cancel_during_stall: cancel ? stallBegin != null && cancel.t_ms >= stallBegin && (stallEnd == null || cancel.t_ms <= stallEnd) : null,
    stall_end_to_dialog_gone_ms: cancel && stallEnd != null && lastGone != null ? lastGone - stallEnd : null,
  });
}
rows.sort((a, b) => a.case_id.localeCompare(b.case_id) || a.run_id.localeCompare(b.run_id));
writeFileSync(join(dir, 'stage-b1-timings.json'), JSON.stringify(rows, null, 2) + '\n');
const show = rows.filter((r) => r.cancel_to_dialog_gone_ms != null || r.kill_to_error_dialog_ms != null || r.error_to_error_dialog_ms != null);
for (const r of show) console.log([r.case_id, r.threading, 'err->dlg', r.error_to_error_dialog_ms, 'kill->dlg', r.kill_to_error_dialog_ms, 'cancel->gone', r.cancel_to_dialog_gone_ms, 'during_stall', r.cancel_during_stall].join(' '));
