// Stage B1 dialog-driver calibration (spec §9.4): operator clicks (D1) versus the targeted UI
// Automation driver (D2u) on the same cases. Equivalent only if outcome classes, target
// verdicts, dialog kinds and observable destination behavior match.
// Usage: node scripts/compare-b1-dialog-drivers.mjs <reports/stage-b1> <out-dir> <case,case,...>
import { readFileSync, readdirSync, writeFileSync, mkdirSync, statSync } from 'node:fs';
import { join } from 'node:path';

const [dir = 'reports/stage-b1', outDir = 'reports/stage-b1/calibration', caseList = 'ERR-01,CANCEL-01b,PROC-01'] = process.argv.slice(2);
const wanted = caseList.split(',');
const read = (p) => JSON.parse(readFileSync(p, 'utf8').replace(/^﻿/, ''));
const recs = readdirSync(dir)
  .filter((f) => f.endsWith('.json') && statSync(join(dir, f)).isFile())
  .map((f) => read(join(dir, f)))
  .filter((r) => r.stage === 'B1' && wanted.includes(r.case_id) && r.result === 'completed');

// Facts that must not depend on who clicks.
function facts(r) {
  const fs = Object.keys(r.observer ?? {}).length ? r : null;
  const dialogKinds = (r.dialogs?.events ?? []).filter((e) => e.event === 'appear').map((e) => (e.progress_bar ? 'progress' : 'error'));
  const english = (r.dialogs?.error_texts ?? []).filter((t) => /[A-Za-z]{3}/.test(t) && t !== 'MainInstructionIcon');
  return {
    report: r.classification?.target_report,
    class: r.classification?.outcome_class,
    file_states: (r.destinations ?? []).flatMap((d) => (d.files ?? []).map((f) => f.state)),
    unexpected_entries: (r.destinations ?? []).flatMap((d) => (d.unexpected_entries ?? []).map((e) => e.name)),
    dialog_kinds: [...new Set(dialogKinds)].sort(),
    error_texts: english,
    performed_drop_effect: (r.performed_drop_effect ?? []).length > 0,
    streams_created: r.streams?.created ?? null,
    all_reached_eof: r.streams?.all_reached_eof ?? null,
    fault_events: r.agent_fault_events ?? [],
    destination_names_seen: fs ? [...new Set(Object.values(r.observer).flatMap((o) => o.names_seen ?? []))] : [],
    clipboard_clean: r.clipboard?.clean_after ?? null,
  };
}
const eq = (a, b) => JSON.stringify(a) === JSON.stringify(b);
const out = { generated: new Date().toISOString(), method: 'spec §9.4: outcome class, target verdict, dialog kinds and destination behavior must match', cases: [] };
for (const c of wanted) {
  const d1 = recs.filter((r) => r.case_id === c && r.dialog_driver === 'D1');
  const d2 = recs.filter((r) => r.case_id === c && r.dialog_driver === 'D2u');
  const pairs = [];
  for (const a of d1) for (const b of d2) {
    const fa = facts(a), fb = facts(b);
    const diffs = Object.keys(fa).filter((k) => !eq(fa[k], fb[k]));
    pairs.push({ d1: a.run_id, d2u: b.run_id, equivalent: diffs.length === 0, differences: diffs.map((k) => ({ field: k, d1: fa[k], d2u: fb[k] })) });
  }
  out.cases.push({ case_id: c, d1_runs: d1.length, d2u_runs: d2.length, pairs, equivalent: pairs.length > 0 && pairs.every((p) => p.equivalent) });
}
out.equivalent = out.cases.every((c) => c.equivalent);
mkdirSync(outDir, { recursive: true });
writeFileSync(join(outDir, 'd1-vs-d2u.json'), JSON.stringify(out, null, 2) + '\n');
const md = ['| Case | D1 runs | D2u runs | Pairs equivalent | Differences |', '|---|---|---|---|---|'];
for (const c of out.cases) {
  const diffs = [...new Set(c.pairs.flatMap((p) => p.differences.map((d) => d.field)))];
  md.push(`| ${c.case_id} | ${c.d1_runs} | ${c.d2u_runs} | ${c.pairs.filter((p) => p.equivalent).length}/${c.pairs.length} | ${diffs.join(', ') || 'none'} |`);
}
writeFileSync(join(outDir, 'd1-vs-d2u.md'), md.join('\n') + '\n');
console.log(md.join('\n'));
console.log('overall equivalent:', out.equivalent);
