import { matrixRows } from '../test/externalinput/matrix.mjs';

const [axis] = process.argv.slice(2);
for (const cell of matrixRows(axis)) {
  const fields = axis === 'control'
    ? [cell.browser, cell.sequence, cell.profileId, cell.domRequired ? 1 : 0]
    : [cell.browser, cell.sequence, cell.profileId, cell.locale];
  process.stdout.write(`${fields.join('\t')}\n`);
}
