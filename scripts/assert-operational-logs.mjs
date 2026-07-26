#!/usr/bin/env node
// Docker operational-log conformance (SoT-40).
// Usage: node scripts/assert-operational-logs.mjs [deployments/artifacts/logs]

import { readFileSync } from 'node:fs';
import { join, resolve } from 'node:path';

const root = resolve(process.argv[2] || 'deployments/artifacts/logs');
const forbidden = [
  'e2e-auditor-token',
  'e2e-operator-token',
  'e2e-approver-token',
  'e2e-dpo-token',
  'development tokens',
];

function lines(name) {
  const path = join(root, name);
  const text = readFileSync(path, 'utf8');
  for (const sentinel of forbidden) {
    if (text.includes(sentinel)) {
      throw new Error(`${name}: forbidden credential sentinel ${sentinel}`);
    }
  }
  const records = text.split(/\r?\n/).filter(Boolean);
  if (records.length === 0) throw new Error(`${name}: no records`);
  for (const [index, line] of records.entries()) {
    if (Buffer.byteLength(`${line}\n`) > 4096) {
      throw new Error(`${name}:${index + 1}: record exceeds 4096 bytes`);
    }
  }
  return records;
}

function plainKeys(service) {
  return lines(`${service}.log`).map((line, index) => {
    if (!/^\d{4}-\d{2}-\d{2}T.*Z (DEBUG|INFO|WARN|ERROR) /.test(line)) {
      throw new Error(`${service}.log:${index + 1}: non-canonical plain record`);
    }
    const event = line.match(/\bevent="([^"]+)"/)?.[1];
    const sequence = line.match(/\bsequence=(\d+)/)?.[1];
    if (!event || !sequence) {
      throw new Error(`${service}.log:${index + 1}: missing event or sequence`);
    }
    return `${sequence}:${event}`;
  });
}

function jsonKeys(service) {
  return lines(`${service}.jsonl`).map((line, index) => {
    let record;
    try {
      record = JSON.parse(line);
    } catch (error) {
      throw new Error(`${service}.jsonl:${index + 1}: invalid JSON: ${error.message}`);
    }
    if (
      record.schema_version !== '1.0.0' ||
      record.service !== service ||
      typeof record.event !== 'string' ||
      !Number.isInteger(record.sequence)
    ) {
      throw new Error(`${service}.jsonl:${index + 1}: invalid canonical envelope`);
    }
    return `${record.sequence}:${record.event}`;
  });
}

for (const service of ['core', 'gate']) {
  const plain = plainKeys(service);
  const jsonl = jsonKeys(service);
  if (plain.length !== jsonl.length || plain.some((key, index) => key !== jsonl[index])) {
    throw new Error(`${service}: plain and JSONL event streams differ`);
  }
  console.log(`PASS ${service}: ${plain.length} matched plain/JSONL records`);
}
