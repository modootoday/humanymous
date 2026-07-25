#!/usr/bin/env node
import { createHash } from 'node:crypto';
import { appendFileSync, mkdirSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { DatabaseSync } from 'node:sqlite';

function arg(name, fallback) {
  const index = process.argv.indexOf(name);
  return index >= 0 && process.argv[index + 1] ? process.argv[index + 1] : fallback;
}

const dbPath = resolve(arg('--db', ''));
const outputPath = resolve(arg('--out', '.agent-runs/hooks/codex-sqlite-live.jsonl'));
const pidPath = resolve(arg('--pid-file', '.agent-runs/hooks/codex-sqlite-live.pid'));
const pollMs = Number.parseInt(arg('--poll-ms', '250'), 10);

if (!dbPath) throw new Error('--db is required');
if (!Number.isFinite(pollMs) || pollMs < 50) throw new Error('--poll-ms must be at least 50');

mkdirSync(dirname(outputPath), { recursive: true });
mkdirSync(dirname(pidPath), { recursive: true });
writeFileSync(pidPath, `${process.pid}\n`, 'utf8');

const db = new DatabaseSync(dbPath, { readOnly: true });
let lastId = db.prepare('SELECT COALESCE(MAX(id), 0) AS id FROM logs').get().id;
const selectRows = db.prepare(`
  SELECT id, ts, ts_nanos, level, target, module_path, file, line,
         feedback_log_body AS body, thread_id, process_uuid
  FROM logs
  WHERE id > ?
    AND (
      (target = 'codex_app_server::outgoing_message'
       AND feedback_log_body LIKE 'app-server event: hook/%')
      OR lower(target) LIKE '%hook%'
      OR lower(feedback_log_body) LIKE '%invalid pre-tool-use%'
    )
  ORDER BY id
`);

function sha256(value) {
  return createHash('sha256').update(value).digest('hex');
}

function classify(row) {
  const body = row.body ?? '';
  const eventMatch = body.match(/app-server event:\s+(\S+)/);
  const statusMatch = body.match(/\bstatus[=:"]+\s*"?([A-Za-z_-]+)/i);
  const exitMatch = body.match(/\bexit(?:_code| code)?[=:"]+\s*"?(-?\d+)/i);
  return {
    id: row.id,
    timestamp: new Date(row.ts * 1000 + Math.floor(row.ts_nanos / 1e6)).toISOString(),
    level: row.level,
    target: row.target,
    modulePath: row.module_path,
    file: row.file,
    line: row.line,
    threadId: row.thread_id,
    processUuid: row.process_uuid,
    event: eventMatch?.[1] ?? null,
    status: statusMatch?.[1] ?? null,
    exitCode: exitMatch ? Number.parseInt(exitMatch[1], 10) : null,
    errorKind: body.includes('invalid pre-tool-use JSON output')
      ? 'invalid-pre-tool-use-json'
      : null,
    bodyBytes: Buffer.byteLength(body),
    bodySha256: sha256(body),
  };
}

appendFileSync(
  outputPath,
  `${JSON.stringify({
    daemon: 'started',
    timestamp: new Date().toISOString(),
    pid: process.pid,
    dbPath,
    startingAfterId: lastId,
    pollMs,
  })}\n`,
  'utf8',
);

const timer = setInterval(() => {
  try {
    const rows = selectRows.all(lastId);
    for (const row of rows) {
      appendFileSync(outputPath, `${JSON.stringify(classify(row))}\n`, 'utf8');
      lastId = row.id;
    }
  } catch (error) {
    appendFileSync(
      outputPath,
      `${JSON.stringify({
        daemon: 'poll-error',
        timestamp: new Date().toISOString(),
        name: error?.name ?? 'Error',
        message: String(error?.message ?? error),
      })}\n`,
      'utf8',
    );
  }
}, pollMs);

function stop(signal) {
  clearInterval(timer);
  appendFileSync(
    outputPath,
    `${JSON.stringify({ daemon: 'stopped', timestamp: new Date().toISOString(), signal })}\n`,
    'utf8',
  );
  db.close();
  process.exit(0);
}

process.on('SIGINT', () => stop('SIGINT'));
process.on('SIGTERM', () => stop('SIGTERM'));
