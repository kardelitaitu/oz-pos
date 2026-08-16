// OTP dashboard end-to-end check (local dev): site on :4321, license
// server on :8090, SMTP sink log path passed via argv.
//
// Usage: node e2e-otp-flow.mjs <smtp-sink-log-path>
//
// Drives headless Chrome over CDP (Node 22 built-in WebSocket), no deps.
import { spawn } from 'node:child_process';
import { readFileSync, appendFileSync } from 'node:fs';
import { setTimeout as sleep } from 'node:timers/promises';

const sinkLog = process.argv[2];
if (!sinkLog) {
  console.error('usage: node e2e-otp-flow.mjs <smtp-sink-log-path>');
  process.exit(2);
}

const CHROME =
  process.env.CHROME_PATH ||
  'C:/Program Files/Google/Chrome/Application/chrome.exe';

// ── launch Chrome headless with remote debugging ────────────────────
// Fresh profile per run so a lingering instance can't hijack the port
// or user-data-dir and serve stale targets.
const profileDir = process.cwd() + '/.e2e-chrome-profile-' + Date.now();
const chrome = spawn(CHROME, [
  '--headless=new',
  '--disable-gpu',
  '--no-first-run',
  '--remote-debugging-port=9333',
  '--user-data-dir=' + profileDir,
  'about:blank',
], { stdio: 'ignore' });

let wsUrl;
for (let i = 0; i < 40; i++) {
  await sleep(250);
  try {
    const res = await fetch('http://127.0.0.1:9333/json/version');
    const j = await res.json();
    wsUrl = j.webSocketDebuggerUrl;
    break;
  } catch { /* not up yet */ }
}
if (!wsUrl) {
  console.error('chrome did not start debugging endpoint');
  chrome.kill();
  process.exit(1);
}
const pages = await (await fetch('http://127.0.0.1:9333/json/list')).json();
const target = pages.find((p) => p.type === 'page' && p.url !== 'chrome://newtab/');
if (!target) {
  console.error('no page target found:', JSON.stringify(pages.map((p) => p.type + ':' + p.url)));
  chrome.kill();
  process.exit(1);
}
wsUrl = target.webSocketDebuggerUrl;

// ── minimal CDP client ──────────────────────────────────────────────
let msgId = 0;
const pending = new Map();
const ws = new WebSocket(wsUrl);
await new Promise((res, rej) => { ws.onopen = res; ws.onerror = rej; });

function send(method, params = {}) {
  const id = ++msgId;
  return new Promise((resolve) => {
    pending.set(id, resolve);
    ws.send(JSON.stringify({ id, method, params }));
  });
}
ws.onmessage = (ev) => {
  const m = JSON.parse(ev.data);
  if (m.id && pending.has(m.id)) {
    pending.get(m.id)(m);
    pending.delete(m.id);
  }
};

const sleepMs = (ms) => sleep(ms);

// ── helpers ─────────────────────────────────────────────────────────
let lastNav = null;
async function navigate(url) {
  await send('Page.enable');
  await send('Page.navigate', { url });
  lastNav = url;
  // Poll until the document actually changed to the target URL (avoids
  // evaluating against a stale about:blank context).
  for (let i = 0; i < 30; i++) {
    const href = await evalJs('location.href');
    if (typeof href === 'string' && href.includes(url)) {
      await sleepMs(1200); // let hydration + fetch settle
      return;
    }
    await sleepMs(250);
  }
  if (process.env.E2E_TRACE) {
    console.log('TRACE navigate timeout for', url, 'href=', await evalJs('location.href'));
  }
}

// Track execution contexts so eval always targets the newest one (avoids
// evaluating against a stale about:blank context after navigation).
let contextId = null;
ws.addEventListener('message', (ev) => {
  const m = JSON.parse(ev.data);
  if (m.method === 'Runtime.executionContextCreated' && m.params.context) {
    contextId = m.params.context.id;
  }
  if (m.method === 'Runtime.executionContextsCleared') {
    contextId = null;
  }
});

async function evalJs(expression) {
  const params = {
    expression,
    awaitPromise: true,
    returnByValue: true,
  };
  if (contextId) params.contextId = contextId;
  const r = await send('Runtime.evaluate', params);
  if (r.result && r.result.exceptionDetails) {
    throw new Error('eval failed: ' + JSON.stringify(r.result.exceptionDetails));
  }
  return r.result && r.result.result ? r.result.result.value : undefined;
}

async function waitForSelector(selector, timeoutMs = 4000) {
  const start = Date.now();
  while (Date.now() - start < timeoutMs) {
    const found = await evalJs(`!!document.querySelector(${JSON.stringify(selector)})`);
    if (found) return true;
    await sleepMs(150);
  }
  return false;
}

async function type(selector, text) {
  await waitForSelector(selector);
  // Typing into the SSR'd input before React hydration finishes is lost:
  // hydration resets the controlled input to its initial state. So set the
  // value, give hydration a beat, then set it again (and verify) so the
  // final value sticks.
  const set = () => evalJs(`(() => {
    const el = document.querySelector(${JSON.stringify(selector)});
    if (!el) return false;
    el.focus();
    const setter = Object.getOwnPropertyDescriptor(window.HTMLInputElement.prototype, 'value').set;
    setter.call(el, ${JSON.stringify(text)});
    el.dispatchEvent(new Event('input', { bubbles: true }));
    return true;
  })()`);
  await set();
  await sleepMs(400);
  await set();
  await sleepMs(150);
  const final = await evalJs(`document.querySelector(${JSON.stringify(selector)})?.value`);
  return final === text;
}

async function click(selector) {
  await waitForSelector(selector);
  return evalJs(`(() => {
    const el = document.querySelector(${JSON.stringify(selector)});
    if (!el) return false;
    el.click();
    return true;
  })()`);
}

async function pageText() {
  return evalJs(`document.body.innerText`);
}

function waitForCode() {
  // read the OTP from the sink log: last "verification code is: NNNNNN"
  const log = readFileSync(sinkLog, 'utf8');
  const matches = [...log.matchAll(/verification code is: (\d{6})/g)];
  return matches.length ? matches[matches.length - 1][1] : null;
}

// ── the flow ────────────────────────────────────────────────────────
const results = [];
const failures = [];
function check(name, ok, detail) {
  results.push({ name, ok, detail });
  if (!ok) failures.push(name);
  console.log(`${ok ? 'PASS' : 'FAIL'}  ${name}${detail ? ' — ' + detail : ''}`);
}

console.log('CDP connected, target:', target.url);
// sanity: evaluate must round-trip
const sanity = await evalJs('1 + 1');
console.log('eval sanity:', sanity);

// 1. login page renders the form
await navigate('http://localhost:4321/en/login/');
const loginText = await pageText();
if (process.env.E2E_TRACE) {
  console.log('TRACE login href=', await evalJs('location.href'), 'ctx=', contextId);
  console.log('TRACE login text=', JSON.stringify((loginText || '').slice(0, 120)));
  console.log('TRACE includes:', !!loginText, typeof loginText,
    String(loginText || '').includes('Sign in to your account'),
    String(loginText || '').includes('Email'));
}
check('login page renders', String(loginText || '').includes('Sign in to your account') && String(loginText || '').includes('Email'), '');

// 2. request a code for the seeded tenant
const before = readFileSync(sinkLog, 'utf8').length;
check('email input found', await type('input[type=email]', 'owner@demo.com'), '');
check('send-code button clicked', await click('button[type=submit]'), '');
await sleepMs(2000);
const afterLen = readFileSync(sinkLog, 'utf8').length;
const code = waitForCode();
if (process.env.E2E_TRACE) {
  const body = await evalJs('document.body.innerText');
  console.log('TRACE sink grew:', afterLen - before, 'bytes; code=', code);
  console.log('TRACE page after send:', JSON.stringify((body || '').slice(0, 150)));
}
check('OTP emailed to tenant', code !== null && afterLen > before, code ? `code=${code}` : 'no code in sink');

// 3. code step appears, enter code
await sleepMs(500);
const stepText = await pageText();
check('code step shown', stepText.includes('verification code'), '');
check('code input found', await type('input[inputmode=numeric]', code), '');
check('verify button clicked', await click('button[type=submit]'), '');
await sleepMs(2500); // redirect to /en/account + /me fetch

// 4. account page shows license data (tier/status rendered capitalized)
const accountText = await pageText();
if (process.env.E2E_TRACE) {
  console.log('TRACE account url=', await evalJs('location.href'));
  console.log('TRACE account text=', JSON.stringify((accountText || '').slice(0, 400)));
}
check('account renders license', accountText.includes('OZ-PRO-DEMO-ABCD-EFGH'), '');
check('account shows tier', /pro/i.test(accountText), '');
if (process.env.E2E_TRACE) {
  console.log('TRACE regex: status=', /activ/i.test(accountText), 'expiry=', /2027-08-01/.test(accountText));
}
// Status renders as "Activated" (capitalized) — /activ/i matches it.
check('account shows status+expiry', /activ/i.test(accountText) && /2027-08-01/.test(accountText), '');

// 5. signed-in session persisted in sessionStorage
const stored = await evalJs(`sessionStorage.getItem('oz_session') || ''`);
check('session token in sessionStorage', stored.length > 20, stored.slice(0, 16) + '…');

// 6. logout clears session + server session. The account page's only
// <button> is the Sign out action; find it by its text content (the
// Playwright-style :has-text() pseudo is NOT valid CSS for querySelector).
const logoutClicked = await evalJs(`(() => {
  const btns = [...document.querySelectorAll('button')];
  const b = btns.find((x) => (x.textContent || '').includes('Sign out')) || btns[0];
  if (!b) return false;
  b.click();
  return true;
})()`);
check('logout button clicked', logoutClicked, '');
await sleepMs(2000);
const afterLogout = await evalJs(`sessionStorage.getItem('oz_session') || ''`);
if (process.env.E2E_TRACE) {
  console.log('TRACE after logout url=', await evalJs('location.href'), 'stored=', JSON.stringify(afterLogout));
}
check('logout clears stored token', afterLogout === '', '');

// 7. account page back to signed-out state
await navigate('http://localhost:4321/en/account/');
await sleepMs(2000);
const anonText = await pageText();
if (process.env.E2E_TRACE) {
  console.log('TRACE anon text=', JSON.stringify((anonText || '').slice(0, 200)));
}
check('account shows signed-out state', anonText.includes("You're not signed in"), '');

console.log('\n── summary ─────────────────────────────');
for (const r of results) console.log(`${r.ok ? '✅' : '❌'} ${r.name}`);
console.log(failures.length ? `\n${failures.length} FAILURES` : '\nALL CHECKS PASSED');

ws.close();
chrome.kill();
try { spawn('taskkill', ['/F', '/T', '/PID', String(chrome.pid)], { stdio: 'ignore' }); } catch {}
process.exit(failures.length ? 1 : 0);
