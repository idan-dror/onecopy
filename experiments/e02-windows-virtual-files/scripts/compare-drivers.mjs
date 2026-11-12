// Compares D1 (operator Ctrl+V) and D2c (automated folder-view Paste) Stage A runs.
// Usage: node scripts/compare-drivers.mjs <reports/stage-a dir> <out-dir> [commit-prefix]
// Only runs whose experiment commit starts with commit-prefix (if given) are compared.
//
// Equivalence (per case) requires identical:
//   - paste-phase call signature (UI polling removed, consecutive duplicates collapsed),
//   - thread behavior (all COM calls on the owner STA),
//   - stream behavior (streams, reads, request sizes, seek pattern, Stat calls, EOF read),
//   - outcomes (names, sizes, digests, clipboard empty after exit, exit code),
//   - error set (HRESULTs other than expected S_FALSE at EOF and DV_E_FORMATETC for unoffered formats).
// UI-polling counts (Preferred DropEffect reads, DropDescription writes, QueryGetData, EnumFormatEtc)
// depend on mouse and keyboard interaction and are reported but not required to match.
import { readFileSync, readdirSync, writeFileSync, mkdirSync, existsSync } from 'node:fs';
import { join } from 'node:path';

const [reportsDir = 'reports/stage-a', outDir = 'reports/stage-a/calibration', commitPrefix = ''] = process.argv.slice(2);
const evidenceDir = 'evidence';
const UI_FORMATS = new Set(['Preferred DropEffect', 'DropDescription']);
const EXPECTED_HR = new Set(['0x00000000', '0x00000001', '0x80040064']);

const records = readdirSync(reportsDir)
  .filter((n) => /^A-0[145]-.*\.json$/.test(n))
  .map((n) => JSON.parse(readFileSync(join(reportsDir, n), 'utf8').replace(/^﻿/, '')))
  .filter((r) => r.env && (!commitPrefix || (r.env.experiment_commit ?? '').startsWith(commitPrefix)))
  .filter((r) => existsSync(join(evidenceDir, r.run_id, 'events.jsonl')));

function analyze(r) {
  const ev = readFileSync(join(evidenceDir, r.run_id, 'events.jsonl'), 'utf8').trim().split('\n').map((l) => JSON.parse(l));
  const owner = ev.find((e) => e.event === 'set_clipboard')?.thread_id;
  const com = ev.filter((e) => /^(get_data|query_get_data|set_data|enum_format_etc|get_data_here|stream_)/.test(e.event));
  const sig = [];
  for (const e of com) {
    if (e.event === 'query_get_data' || e.event === 'enum_format_etc' || e.event === 'stream_read') continue;
    if ((e.event === 'get_data' || e.event === 'set_data') && UI_FORMATS.has(e.format)) continue;
    const key = e.event.startsWith('stream_')
      ? `${e.event}${e.event === 'stream_seek' ? `(origin=${e.origin},move=${e.move})` : ''}`
      : `${e.event}:${e.format}${e.event === 'get_data' && e.format === 'FileContents' ? `#${e.lindex}` : ''}${e.event === 'get_data' ? (e.ok ? '' : '!') : ''}`;
    if (sig[sig.length - 1] !== key) sig.push(key);
  }
  const reads = ev.filter((e) => e.event === 'stream_read');
  const errors = [...new Set(com.filter((e) => e.hr && !EXPECTED_HR.has(e.hr)).map((e) => `${e.event}:${e.format ?? ''}:${e.hr}`))];
  const s = r.agent_summary ?? {};
  const files = (r.destinations ?? []).flatMap((d) => d.files.map((f) => `${f.name}:${f.size}:${f.digest_match}`)).sort();
  return {
    run_id: r.run_id,
    driver: r.driver,
    result: r.result,
    commit: (r.env.experiment_commit ?? '').slice(0, 7),
    signature: sig,
    threads: {
      all_on_owner_sta: com.every((e) => e.thread_id === owner),
      distinct_com_threads: new Set(com.map((e) => e.thread_id)).size,
    },
    streams: {
      created: s.streams_created,
      released: s.streams_released,
      read_calls: s.read_calls,
      request_sizes: [...new Set(reads.map((e) => e.requested))].sort((a, b) => a - b),
      short_reads: s.short_reads,
      stat_calls: s.calls?.['IStream::Stat'] ?? 0,
      seek_pattern: ev.filter((e) => e.event === 'stream_seek').map((e) => `${e.origin}:${e.move}`).join(','),
      eof_read_all: (s.per_stream ?? []).every((x) => x.eof_read || x.advertised_size === 0),
      lifetime_ms: (s.per_stream ?? []).length ? Math.round((s.last_stream_release_ms ?? 0) - (s.first_contents_get_data_ms ?? 0)) : null,
    },
    outcome: {
      files,
      clipboard_empty_after_exit: (r.clipboard_after_exit?.formats ?? ['?']).length === 0,
      clipboard_lost: s.clipboard_lost_ms ?? null,
      exit_code: r.agent_exit_code,
      window: (r.explorer_windows ?? []).map((w) => ({ top_preexisting: w.top_preexisting, tabs_on_top: w.tabs_on_top, close_decision: w.close_decision })),
    },
    errors,
    ui_polling: {
      preferred_dropeffect_gets: com.filter((e) => e.event === 'get_data' && e.format === 'Preferred DropEffect').length,
      dropdescription_sets: com.filter((e) => e.event === 'set_data' && e.format === 'DropDescription').length,
      query_get_data: com.filter((e) => e.event === 'query_get_data').length,
      enum_format_etc: com.filter((e) => e.event === 'enum_format_etc').length,
    },
    harness: { interference: r.interference ?? [], exception: r.harness?.exception ?? null },
  };
}

const runs = records.map(analyze);
const cases = {};
for (const a of runs) (cases[a.run_id.slice(0, 4)] ??= []).push(a);

const same = (x, y) => JSON.stringify(x) === JSON.stringify(y);
const comparisons = [];
for (const [caseId, list] of Object.entries(cases).sort()) {
  const d1 = list.filter((x) => x.driver === 'D1');
  const d2 = list.filter((x) => x.driver === 'D2c');
  for (const a of d1) {
    for (const b of d2) {
      const diffs = [];
      if (!same(a.signature, b.signature)) diffs.push({ field: 'signature', d1: a.signature, d2c: b.signature });
      if (!same(a.threads, b.threads)) diffs.push({ field: 'threads', d1: a.threads, d2c: b.threads });
      const sa = { ...a.streams, lifetime_ms: undefined }; const sb = { ...b.streams, lifetime_ms: undefined };
      if (!same(sa, sb)) diffs.push({ field: 'streams', d1: a.streams, d2c: b.streams });
      const oa = { ...a.outcome, window: undefined }; const ob = { ...b.outcome, window: undefined };
      if (!same(oa, ob)) diffs.push({ field: 'outcome', d1: a.outcome, d2c: b.outcome });
      if (!same(a.errors, b.errors)) diffs.push({ field: 'errors', d1: a.errors, d2c: b.errors });
      comparisons.push({ case: caseId, d1: a.run_id, d2c: b.run_id, equivalent: diffs.length === 0, diffs });
    }
  }
}

const summary = {
  generated: new Date().toISOString(),
  commit_filter: commitPrefix || null,
  runs: runs.length,
  comparisons: comparisons.length,
  all_equivalent: comparisons.length > 0 && comparisons.every((c) => c.equivalent),
  by_case: Object.fromEntries(Object.entries(cases).map(([k, v]) => [k, { d1: v.filter((x) => x.driver === 'D1').length, d2c: v.filter((x) => x.driver === 'D2c').length }])),
};
mkdirSync(outDir, { recursive: true });
writeFileSync(join(outDir, 'd1-vs-d2c.json'), JSON.stringify({ summary, comparisons, runs }, null, 2));

const md = [
  '| Case | D1 run | D2c run | Equivalent | Differences |',
  '|---|---|---|---|---|',
  ...comparisons.map((c) => `| ${c.case} | ${c.d1} | ${c.d2c} | ${c.equivalent ? 'yes' : '**no**'} | ${c.diffs.map((d) => d.field).join(', ') || '—'} |`),
  '',
  '| Run | Driver | Result | Signature length | Reads | Request sizes | Seeks | Stat | All on STA | Lifetime ms | UI polling (DropEffect gets / DropDescription sets / QueryGetData) | Window (preexisting / tabs / close) |',
  '|---|---|---|---|---|---|---|---|---|---|---|---|',
  ...runs.map((a) => `| ${a.run_id} | ${a.driver} | ${a.result} | ${a.signature.length} | ${a.streams.read_calls} | ${a.streams.request_sizes.join('/')} | ${a.streams.seek_pattern} | ${a.streams.stat_calls} | ${a.threads.all_on_owner_sta} | ${a.streams.lifetime_ms} | ${a.ui_polling.preferred_dropeffect_gets} / ${a.ui_polling.dropdescription_sets} / ${a.ui_polling.query_get_data} | ${a.outcome.window.map((w) => `${w.top_preexisting}/${w.tabs_on_top}/${w.close_decision}`).join('; ')} |`),
];
writeFileSync(join(outDir, 'd1-vs-d2c.md'), md.join('\n') + '\n');
console.log(md.join('\n'));
console.log('\n' + JSON.stringify(summary, null, 2));
