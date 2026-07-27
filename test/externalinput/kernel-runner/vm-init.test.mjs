import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

const initUrl = new URL(
  '../../../deployments/external-input/kernel-runner/vm-init.sh',
  import.meta.url,
);

test('guest loads configfs before using its sysfs-created mountpoint', async () => {
  const source = await readFile(initUrl, 'utf8');
  const load = source.indexOf('modprobe configfs');
  const requireMountpoint = source.indexOf(
    '[[ -d /sys/kernel/config ]] || fail configfs-mountpoint-missing',
  );
  const mount = source.indexOf(
    'mountpoint -q /sys/kernel/config || mount -t configfs configfs /sys/kernel/config',
  );

  assert.notEqual(load, -1);
  assert.ok(load < requireMountpoint);
  assert.ok(requireMountpoint < mount);
  assert.doesNotMatch(
    source.slice(0, load),
    /mkdir[^\n]*\/sys\/kernel\/config/,
    'sysfs rejects arbitrary mkdir before the configfs module creates its mountpoint',
  );
});

test('guest creates a private runtime directory for its non-root supervisor', async () => {
  const source = await readFile(initUrl, 'utf8');
  const create = source.indexOf(
    'install -d -m 0700 -o humanymous -g humanymous /run/user/12000',
  );
  const delegate = source.indexOf('XDG_RUNTIME_DIR=/run/user/12000');
  const runAsMeasurementUser = source.indexOf('runuser -u humanymous -- env');

  assert.notEqual(create, -1);
  assert.ok(create < runAsMeasurementUser);
  assert.ok(runAsMeasurementUser < delegate);
});
