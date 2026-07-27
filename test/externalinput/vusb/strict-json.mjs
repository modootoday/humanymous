// Minimal strict JSON parser used at the profile/catalog trust boundary.
// JSON.parse silently accepts duplicate object keys; this parser rejects them.

export function parseStrictJson(source, label = 'JSON') {
  if (typeof source !== 'string') throw new TypeError(`${label} must be text`);
  let offset = 0;

  const fail = (message) => {
    throw new TypeError(`${label} ${message} at byte ${Buffer.byteLength(source.slice(0, offset))}`);
  };
  const whitespace = () => {
    while (offset < source.length && /[\u0020\u000a\u000d\u0009]/.test(source[offset])) offset += 1;
  };
  const string = () => {
    if (source[offset] !== '"') fail('expected string');
    const start = offset++;
    while (offset < source.length) {
      const char = source[offset++];
      if (char === '"') {
        try {
          return JSON.parse(source.slice(start, offset));
        } catch {
          fail('contains an invalid string');
        }
      }
      if (char === '\\') {
        if (offset >= source.length) fail('contains an unterminated escape');
        if (source[offset] === 'u') {
          offset += 1;
          if (!/^[0-9a-fA-F]{4}$/.test(source.slice(offset, offset + 4))) {
            fail('contains an invalid unicode escape');
          }
          offset += 4;
        } else if ('"\\/bfnrt'.includes(source[offset])) {
          offset += 1;
        } else {
          fail('contains an invalid escape');
        }
      } else if (char.charCodeAt(0) < 0x20) {
        fail('contains a control character');
      }
    }
    fail('contains an unterminated string');
  };

  const value = () => {
    whitespace();
    const char = source[offset];
    if (char === '{') {
      offset += 1;
      const result = {};
      const keys = new Set();
      whitespace();
      if (source[offset] === '}') {
        offset += 1;
        return result;
      }
      for (;;) {
        whitespace();
        const key = string();
        if (keys.has(key)) fail(`contains duplicate key ${JSON.stringify(key)}`);
        keys.add(key);
        whitespace();
        if (source[offset++] !== ':') fail('expected colon');
        result[key] = value();
        whitespace();
        const separator = source[offset++];
        if (separator === '}') return result;
        if (separator !== ',') fail('expected comma or object end');
      }
    }
    if (char === '[') {
      offset += 1;
      const result = [];
      whitespace();
      if (source[offset] === ']') {
        offset += 1;
        return result;
      }
      for (;;) {
        result.push(value());
        whitespace();
        const separator = source[offset++];
        if (separator === ']') return result;
        if (separator !== ',') fail('expected comma or array end');
      }
    }
    if (char === '"') return string();
    for (const [literal, parsed] of [['true', true], ['false', false], ['null', null]]) {
      if (source.startsWith(literal, offset)) {
        offset += literal.length;
        return parsed;
      }
    }
    const number = source.slice(offset).match(/^-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?/);
    if (number) {
      offset += number[0].length;
      return Number(number[0]);
    }
    fail('contains an invalid value');
  };

  const parsed = value();
  whitespace();
  if (offset !== source.length) fail('contains trailing content');
  return parsed;
}

