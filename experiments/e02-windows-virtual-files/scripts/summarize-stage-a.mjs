// Summarizes reports/stage-a/*.json run records into a Markdown table and a JSON roll-up.
// Usage: node scripts/summarize-stage-a.mjs [reports/stage-a]
import { readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';

const dir = process.argv[2] ?? 'reports/stage-a';
const records = readdirSync(dir)
  .filter((n) => /^A-0\d-.*\.json$/.test(n))
  .map((n) => JSON.parse(readFileSync(join(dir, n), 'utf8').replace(/^﻿/, '')))
  .sort((a, b) => a.run_id.localeCompare(b.run_id));

const mib = (b) => (b / 1048576).toFixed(2);
const rows = [];
const byCase = {};
for (const r of records) {
  const s = r.agent_summary;
  const files = r.destinations.flatMap((d) => d.files);
  const streams = s.per_stream ?? [];
  const row = {
    run_id: r.run_id,
    case_id: r.case_id,
    result: r.result,
    driver: r.driver,
    commit: (r.env.experiment_commit ?? '').slice(0, 7),
    dirty: r.env.experiment_tree_dirty,
    files_ok: files.filter((f) => f.digest_match === true || (f.expected_size === 0 && f.size === 0)).length,
    files_total: files.length,
    streams_created: s.streams_created,
    streams_released: s.streams_released,
    bytes_served: s.bytes_served,
    read_calls: s.read_calls,
    max_request: Math.max(0, ...streams.map((x) => x.max_request ?? 0)),
    stat_calls: s.calls['IStream::Stat'] ?? 0,
    seeks: s.calls['IStream::Seek'] ?? 0,
    other_stream_calls: Object.keys(s.calls).filter((k) => k.startsWith('IStream::') && !['IStream::Seek', 'IStream::Stat'].includes(k)),
    eof_read_all: streams.length > 0 && streams.every((x) => x.eof_read || x.advertised_size === 0),
    peak_private_mib: Number(mib(s.peak_private_bytes)),
    paste_to_last_release_ms: s.last_stream_release_ms && s.first_descriptor_get_data_ms ? Math.round(s.last_stream_release_ms - s.first_descriptor_get_data_ms) : null,
    zone_identifier_present: files.some((f) => f.zone_identifier && f.zone_identifier.present),
    unexpected_entries: r.destinations.reduce((n, d) => n + d.unexpected_entries.length, 0),
    observer_names: [...new Set(Object.values(r.observer).flatMap((o) => o.names_seen))],
    clipboard_after_exit_formats: r.clipboard_after_exit.formats.length,
    marker_after_exit: r.clipboard_after_exit.has_onecopy_marker,
    exit_code: r.agent_exit_code,
    owner_thread_id: s.owner_thread_id ?? null,
    com_call_threads: s.com_call_threads ?? null,
    com_calls_on_owner_only: s.owner_thread_id !== undefined && s.com_call_threads !== undefined ? Object.keys(s.com_call_threads).every((t) => Number(t) === s.owner_thread_id) : null,
  };
  rows.push(row);
  (byCase[r.case_id] ??= []).push(row);
}

const lines = [
  '| Run | Result | Files OK | Streams (created/released) | Bytes served | Reads | Max read req | Stat | Seek | Peak private MiB | Descriptor→last release ms | Zone.Identifier | Clipboard after exit |',
  '|---|---|---|---|---|---|---|---|---|---|---|---|---|',
  ...rows.map((r) => `| ${r.run_id} | ${r.result} | ${r.files_ok}/${r.files_total} | ${r.streams_created}/${r.streams_released} | ${r.bytes_served} | ${r.read_calls} | ${r.max_request} | ${r.stat_calls} | ${r.seeks} | ${r.peak_private_mib} | ${r.paste_to_last_release_ms ?? 'n/a'} | ${r.zone_identifier_present ? 'present' : 'absent'} | ${r.clipboard_after_exit_formats === 0 && !r.marker_after_exit ? 'empty' : 'NOT empty'} |`),
];
const peaks = Object.fromEntries(Object.entries(byCase).map(([k, v]) => [k, Math.max(...v.map((x) => x.peak_private_mib))]));
const rollup = {
  runs: rows.length,
  passed: rows.filter((r) => r.result === 'pass').length,
  commits: [...new Set(rows.map((r) => r.commit))],
  any_dirty: rows.some((r) => r.dirty),
  peak_private_mib_by_case: peaks,
  sa4_a04_minus_a01_mib: peaks['A-04'] !== undefined && peaks['A-01'] !== undefined ? Number((peaks['A-04'] - peaks['A-01']).toFixed(2)) : null,
  any_other_stream_calls: rows.some((r) => r.other_stream_calls.length > 0),
  any_zone_identifier: rows.some((r) => r.zone_identifier_present),
  any_unexpected_entries: rows.some((r) => r.unexpected_entries > 0),
  observer_names_only_final: rows.every((r) => r.observer_names.every((n) => /^e02-A-0\d-f\d\.bin$/.test(n))),
  all_clipboards_empty_after_exit: rows.every((r) => r.clipboard_after_exit_formats === 0 && !r.marker_after_exit),
  all_streams_released: rows.every((r) => r.streams_created === r.streams_released),
  all_eof_read: rows.every((r) => r.eof_read_all),
  exit_codes: [...new Set(rows.map((r) => r.exit_code))],
  com_calls_on_owner_sta_only: rows.every((r) => r.com_calls_on_owner_only === true) ? true : rows.some((r) => r.com_calls_on_owner_only === null) ? "not recorded" : false,
};
writeFileSync(join(dir, 'stage-a-rollup.json'), JSON.stringify({ rollup, rows }, null, 2));
console.log(lines.join('\n'));
console.log('\n' + JSON.stringify(rollup, null, 2));
