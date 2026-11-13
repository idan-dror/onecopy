// Stage B1 roll-up: reads the durable run records and writes a per-case outcome matrix.
// Usage: node scripts/summarize-stage-b1.mjs <reports/stage-b1> [commit-prefix]
// Only top-level records are read (superseded/, regression-a/ and safety/ are excluded).
import { readFileSync, readdirSync, writeFileSync, statSync } from 'node:fs';
import { join } from 'node:path';

const dir = process.argv[2] ?? 'reports/stage-b1';
const commitPrefix = process.argv[3] ?? '';
const read = (p) => JSON.parse(readFileSync(p, 'utf8').replace(/^﻿/, ''));
const recs = readdirSync(dir)
  .filter((f) => f.endsWith('.json') && !f.startsWith('stage-b1-') && statSync(join(dir, f)).isFile())
  .map((f) => read(join(dir, f)))
  .filter((r) => r.stage === 'B1')
  .filter((r) => !commitPrefix || (r.env?.experiment_commit ?? '').startsWith(commitPrefix));

const order = ['O5', 'O4', 'O3', 'O2', 'O1', 'C', 'S', 'N'];
const worst = (a, b) => (order.indexOf(a) <= order.indexOf(b) ? a : b);
const tl = (r, ev) => r.timeline?.find((t) => t.event === ev)?.t_ms ?? null;

function runFacts(r) {
  const files = (r.destinations ?? []).flatMap((d) => d.files ?? []);
  const extra = (r.destinations ?? []).flatMap((d) => d.unexpected_entries ?? []);
  const s = r.agent_summary ?? {};
  const threads = Object.keys(s.com_call_threads ?? {});
  const paste = tl(r, 'paste');
  const dialogAppear = r.timeline?.find((t) => t.source === 'dialog' && t.event === 'appear')?.t_ms ?? null;
  const fault = r.timeline?.find((t) => t.source === 'agent' && ['error_returned', 'eof_returned', 'self_kill', 'stall_begin', 'read_timeout', 'mark'].includes(t.event));
  const fsEvents = Object.values(r.observer ?? {}).flatMap((o) => o.names_seen ?? []);
  return {
    run_id: r.run_id,
    commit: (r.env?.experiment_commit ?? '').slice(0, 7),
    dirty: r.env?.experiment_tree_dirty ?? null,
    driver: `${r.driver}/${r.dialog_driver}`,
    threading: r.threading,
    result: r.result,
    report: r.classification?.target_report,
    class: r.classification?.outcome_class,
    expect_met: r.classification?.expectation_met,
    file_states: files.map((f) => f.state),
    sizes: files.map((f) => f.size),
    mistakable_incomplete: files.some((f) => f.mistakable_as_complete && !['complete_correct', 'original_preserved'].includes(f.state)),
    unexpected_entries: extra.map((e) => ({ name: e.name, size: e.size, prefix_of: e.synthetic_prefix_of_file })),
    error_texts: r.dialogs?.error_texts ?? [],
    progress_dialog: r.dialogs?.progress_dialog_seen ?? null,
    performed_drop_effect: r.performed_drop_effect ?? [],
    streams: r.streams,
    bytes_served: s.bytes_served ?? null,
    failed_reads: s.failed_reads ?? null,
    com_threads: threads.length,
    all_calls_on_owner: threads.length > 0 ? threads.every((t) => Number(t) === s.owner_thread_id) : null,
    os_peak_commit_mib: s.os_peak_commit_bytes ? +(s.os_peak_commit_bytes / 1048576).toFixed(2) : null,
    paste_to_fault_ms: paste != null && fault ? fault.t_ms - paste : null,
    fault_to_dialog_ms: fault && dialogAppear != null ? dialogAppear - fault.t_ms : null,
    actions: (r.dialog_actions ?? []).map((a) => `${a.action}:${a.result?.invoked ?? a.result?.refused ?? '?'}`),
    timed_out: r.progress?.timed_out ?? null,
    seconds: r.progress?.seconds ?? null,
    explorer_lost: (r.explorer_processes?.lost ?? []).length,
    clipboard_clean: r.clipboard?.clean_after ?? null,
    clipboard_release: r.clipboard?.release?.identified_as_this_run ?? null,
    clipboard_persisted: r.clipboard?.release?.available ?? null,
    clipboard_replace: r.clipboard?.replacement ?? null,
    probe: r.clipboard?.probe ?? null,
    zone_ids: files.map((f) => f.zone_identifier?.zone_id ?? (f.zone_identifier?.present ? 'present' : null)),
    observed_names: [...new Set(fsEvents)],
    interference: r.interference ?? [],
  };
}

const byCase = {};
for (const r of recs) (byCase[r.case_id] ??= { case_id: r.case_id, description: r.description, expect: r.case?.harness?.expect, runs: [] }).runs.push(runFacts(r));
const cases = Object.values(byCase).sort((a, b) => a.case_id.localeCompare(b.case_id));
for (const c of cases) {
  c.runs.sort((a, b) => a.run_id.localeCompare(b.run_id));
  const done = c.runs.filter((r) => r.result === 'completed');
  c.completed = done.length;
  c.inconclusive = c.runs.length - done.length;
  c.worst_class = done.reduce((w, r) => worst(w, r.class ?? 'N'), 'N');
  c.classes = [...new Set(done.map((r) => r.class))];
  c.reports = [...new Set(done.map((r) => r.report))];
  c.consistent = c.classes.length <= 1 && c.reports.length <= 1;
}
const rollup = {
  generated: new Date().toISOString(),
  filter_commit_prefix: commitPrefix || null,
  records: recs.length,
  completed: recs.filter((r) => r.result === 'completed').length,
  inconclusive: recs.filter((r) => r.result !== 'completed').length,
  class_counts: Object.fromEntries(order.map((o) => [o, recs.filter((r) => r.result === 'completed' && r.classification?.outcome_class === o).length])),
  false_success_runs: recs.filter((r) => ['O3', 'O4'].includes(r.classification?.outcome_class)).map((r) => r.run_id),
  hang_runs: recs.filter((r) => r.classification?.outcome_class === 'O5').map((r) => r.run_id),
  dirty_runs: recs.filter((r) => r.env?.experiment_tree_dirty).map((r) => r.run_id),
  cases,
};
writeFileSync(join(dir, 'stage-b1-rollup.json'), JSON.stringify(rollup, null, 2) + '\n');

const md = [];
md.push('| Case | Runs | Target report | Class | Destination | Notes |', '|---|---|---|---|---|---|');
for (const c of cases) {
  const states = [...new Set(c.runs.flatMap((r) => r.file_states))].join(', ');
  const notes = [];
  if (!c.consistent) notes.push('**inconsistent across repeats**');
  if (c.inconclusive) notes.push(`${c.inconclusive} inconclusive`);
  const texts = [...new Set(c.runs.flatMap((r) => r.error_texts).filter((t) => /[A-Za-z]{3}/.test(t) && !/^MainInstructionIcon$/.test(t)))];
  if (texts.length) notes.push(`error text: ${texts.slice(0, 2).join(' / ')}`);
  md.push(`| ${c.case_id} | ${c.completed}/${c.runs.length} | ${c.reports.join(', ')} | ${c.classes.join(', ')} | ${states} | ${notes.join('; ')} |`);
}
writeFileSync(join(dir, 'stage-b1-rollup.md'), md.join('\n') + '\n');
console.log(`records ${rollup.records} completed ${rollup.completed} inconclusive ${rollup.inconclusive}`);
console.log(JSON.stringify(rollup.class_counts));
console.log(md.join('\n'));
