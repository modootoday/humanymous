const FIXTURES = Object.freeze({
  'ko-KR': Object.freeze({ expected: '한글' }),
  'zh-CN': Object.freeze({ expected: '中文' }),
  'ja-JP': Object.freeze({ expected: '日本語' }),
});

export function startImeFixture(locale) {
  const fixture = FIXTURES[locale];
  if (!fixture) throw new TypeError('IME locale is not allowlisted');

  for (const section of document.querySelectorAll('main > section')) section.hidden = true;
  const step = document.getElementById('ime-step');
  const input = document.getElementById('ime-input');
  const status = document.getElementById('ime-status');
  const statusText = document.getElementById('ime-status-text');
  step.hidden = false;
  document.documentElement.lang = locale;

  let compositionStarted = false;
  let compositionUpdates = 0;
  let compositionEnded = false;
  let trustedInputs = 0;
  let untrustedEvents = 0;

  const setStatus = (name, text) => {
    status.className = `ime-status ime-${name}`;
    statusText.textContent = text;
  };
  const trusted = (event) => {
    if (event.isTrusted === true) return true;
    untrustedEvents += 1;
    setStatus('failed', 'Synthetic input event rejected');
    return false;
  };

  input.addEventListener('compositionstart', (event) => {
    if (!trusted(event)) return;
    compositionStarted = true;
    setStatus('composing', 'Native composition in progress');
  });
  input.addEventListener('compositionupdate', (event) => {
    if (!trusted(event)) return;
    compositionUpdates += 1;
    if (compositionStarted) setStatus('composing', 'Native composition in progress');
  });
  input.addEventListener('compositionend', (event) => {
    if (!trusted(event)) return;
    compositionEnded = true;
  });
  input.addEventListener('beforeinput', (event) => {
    if (trusted(event)) trustedInputs += 1;
  });
  input.addEventListener('input', (event) => {
    if (!trusted(event)) return;
    trustedInputs += 1;
    const actual = input.value;
    const normalized = actual.normalize('NFC');
    if (compositionStarted && compositionUpdates > 0 && compositionEnded &&
        trustedInputs > 0 && untrustedEvents === 0 &&
        actual === normalized && normalized === fixture.expected) {
      setStatus('committed', 'Native composition committed');
    }
  });
}
