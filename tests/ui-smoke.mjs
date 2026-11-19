// Manual browser smoke test for the settings shell.
//
// Start a dev server and a headless chromium with remote debugging:
//   NIX_WEB_GUI_SKIP_METADATA=1 ./result/bin/nix-web-gui serve \
//     --dev-auth --dev-password secret --flake <fixture> --page Accounts ...
//   chromium --headless=new --no-sandbox --remote-debugging-port=9223 ...
// Then run:
//   node tests/ui-smoke.mjs
//
// The fixture must declare users, keybinds, bar, and a text option.
import { existsSync, readFileSync } from 'node:fs';

const base = process.env.NWG_BASE ?? 'http://127.0.0.1:8899';
const cdp = process.env.NWG_CDP ?? 'http://127.0.0.1:9223';
const fixture = process.env.NWG_FIXTURE ?? '';
const username = process.env.NWG_USER ?? 'den';
const password = process.env.NWG_PASSWORD ?? 'secret';

const loginResponse = await fetch(`${base}/api/login`, {
  method: 'POST',
  headers: { 'content-type': 'application/json' },
  body: JSON.stringify({ username, password }),
});
if (!loginResponse.ok) throw new Error(`login failed: ${loginResponse.status}`);
const login = await loginResponse.json();
const cookie = loginResponse.headers.get('set-cookie').split(';')[0];
const session = cookie.split('=').slice(1).join('=');

const target = await fetch(`${cdp}/json/new?about:blank`, { method: 'PUT' }).then((response) => response.json());
const socket = new WebSocket(target.webSocketDebuggerUrl);
await new Promise((resolve) => { socket.onopen = resolve; });

let nextId = 0;
const pending = new Map();
const errors = [];
let dialogPrompt = null;
socket.onmessage = (event) => {
  const message = JSON.parse(event.data);
  if (message.id && pending.has(message.id)) {
    pending.get(message.id)(message);
    pending.delete(message.id);
    return;
  }
  if (message.method === 'Page.javascriptDialogOpening') {
    const params = { accept: true };
    if (dialogPrompt != null) params.promptText = dialogPrompt;
    send('Page.handleJavaScriptDialog', params);
    dialogPrompt = null;
  }
  if (message.method === 'Runtime.exceptionThrown') {
    errors.push(message.params.exceptionDetails.exception?.description ?? message.params.exceptionDetails.text ?? 'exception');
  }
  if (message.method === 'Runtime.consoleAPICalled' && message.params.type === 'error') {
    errors.push(message.params.args.map((arg) => arg.value ?? arg.description).join(' '));
  }
};
function send(method, params = {}) {
  return new Promise((resolve) => {
    const id = ++nextId;
    pending.set(id, resolve);
    socket.send(JSON.stringify({ id, method, params }));
  });
}
const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
async function evaluate(expression) {
  const result = await send('Runtime.evaluate', { expression, returnByValue: true, awaitPromise: true });
  if (result.result?.exceptionDetails) {
    const detail = result.result.exceptionDetails.exception?.description ?? result.result.exceptionDetails.text;
    throw new Error(detail);
  }
  return result.result?.result?.value;
}

await send('Runtime.enable');
await send('Page.enable');
await send('Network.enable');
await send('Network.setCacheDisabled', { cacheDisabled: true });
const origin = new URL(base).origin;
await send('Network.setCookie', {
  name: 'nix_web_gui_session',
  value: session,
  url: `${origin}/`,
  path: '/',
  httpOnly: true,
  sameSite: 'Strict',
});
await send('Page.navigate', { url: `${origin}/` });
await sleep(6000);

const body = await evaluate(
  'document.body.innerText + "\\n" + [...document.querySelectorAll("input,textarea,select")].map((input) => input.value).join("\\n")',
);
for (const item of ['Users', 'alice', 'Add row', 'Binds', 'reload', 'Bar', 'clock', 'Monitors', 'eDP-1', 'Grid', 'clock', 'Hostname']) {
  if (!body.includes(item)) {
    console.error(body.slice(0, 1500));
    throw new Error(`missing text: ${item}`);
  }
}
const cards = await evaluate('document.querySelectorAll(".row-card").length');
const csvFields = await evaluate('document.querySelectorAll("input.csv-input").length');
const monitorTiles = await evaluate('document.querySelectorAll(".monitor-tile").length');
const gridHandles = await evaluate('document.querySelectorAll(".grid-handle").length');
if (cards < 4) throw new Error(`expected card rows, saw ${cards}`);
if (monitorTiles < 1) throw new Error('expected a monitor canvas tile');
if (gridHandles < 1) throw new Error('expected a grid card handle');
if (csvFields < 2) throw new Error(`expected keybind CSV fields, saw ${csvFields}`);

const users = `[...document.querySelectorAll('.card-collection')].find((collection) => collection.querySelector('h3')?.textContent.trim() === 'Users')`;
if (!(await evaluate(`Boolean(${users})`))) throw new Error('Users collection missing');

await evaluate(`(${users}).querySelector('button').click()`);
await sleep(300);
await evaluate(`(() => {
  const panel = [...document.querySelectorAll('.panel')].find((item) => item.textContent.includes('Add row to Users'));
  const input = panel.querySelector('input');
  Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set.call(input, 'bob');
  input.dispatchEvent(new Event('input', { bubbles: true }));
  panel.querySelector('button[type=submit]').click();
})()`);
await sleep(1200);
let titles = await evaluate('[...document.querySelectorAll(".row-card strong")].map((item) => item.textContent.trim())');
if (!titles.includes('bob')) throw new Error('add row did not create bob');

dialogPrompt = 'carol';
await evaluate(`(() => {
  const card = [...document.querySelectorAll('.row-card')].find((item) => item.querySelector('strong')?.textContent.trim() === 'bob');
  [...card.querySelectorAll('button')].find((button) => button.textContent.trim() === 'Rename').click();
})()`);
await sleep(1200);
titles = await evaluate('[...document.querySelectorAll(".row-card strong")].map((item) => item.textContent.trim())');
if (!titles.includes('carol') || titles.includes('bob')) throw new Error('rename failed');

dialogPrompt = null;
await evaluate(`(() => {
  const card = [...document.querySelectorAll('.row-card')].find((item) => item.querySelector('strong')?.textContent.trim() === 'carol');
  [...card.querySelectorAll('button')].find((button) => button.textContent.trim() === 'Remove').click();
})()`);
await sleep(1200);
titles = await evaluate('[...document.querySelectorAll(".row-card strong")].map((item) => item.textContent.trim())');
if (titles.includes('carol')) throw new Error('remove failed');

const before = await evaluate('Number(document.querySelector(\'input[id$="-xPos"]\')?.value ?? 0)');
await evaluate(`(() => {
  const tile = document.querySelector('.monitor-tile');
  const rect = tile.getBoundingClientRect();
  const x = rect.x + rect.width / 2;
  const y = rect.y + rect.height / 2;
  const event = (type, clientX) => new PointerEvent(type, { bubbles: true, clientX, clientY: y, pointerId: 1, pointerType: 'mouse', button: 0, buttons: type === 'pointerup' ? 0 : 1 });
  tile.dispatchEvent(event('pointerdown', x));
  tile.dispatchEvent(event('pointermove', x + 30));
  tile.dispatchEvent(event('pointerup', x + 30));
})()`);
await sleep(600);
const after = await evaluate('Number(document.querySelector(\'input[id$="-xPos"]\')?.value ?? 0)');
if (!(after > before)) throw new Error(`monitor drag did not update xPos: ${before} -> ${after}`);
const gridBefore = await evaluate('Number(document.querySelector(\'input[id$="-col"]\')?.value ?? 0)');
await evaluate(`(() => {
  const handle = document.querySelector('.grid-handle');
  const rect = handle.getBoundingClientRect();
  const event = (type, clientX) => new PointerEvent(type, { bubbles: true, clientX, clientY: rect.y, pointerId: 2, pointerType: 'mouse', button: 0, buttons: type === 'pointerup' ? 0 : 1 });
  handle.dispatchEvent(event('pointerdown', rect.x));
  handle.dispatchEvent(event('pointermove', rect.x + 120));
  handle.dispatchEvent(event('pointerup', rect.x + 120));
})()`);
await sleep(600);
const gridAfter = await evaluate('Number(document.querySelector(\'input[id$="-col"]\')?.value ?? 0)');
if (!(gridAfter > gridBefore)) throw new Error(`grid drag did not update col: ${gridBefore} -> ${gridAfter}`);

async function clickText(text) {
  await evaluate(`(() => {
    const button = [...document.querySelectorAll('button')]
      .find((item) => item.textContent.trim() === ${JSON.stringify(text)});
    if (!button) throw new Error('button not found: ' + ${JSON.stringify(text)});
    button.click();
  })()`);
}

async function setInput(id, value) {
  await evaluate(`(() => {
    const input = document.getElementById(${JSON.stringify(id)});
    if (!input) throw new Error('input not found: ' + ${JSON.stringify(id)});
    const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set;
    setter.call(input, ${JSON.stringify(value)});
    input.dispatchEvent(new Event('input', { bubbles: true }));
  })()`);
}

await setInput('control-networking-hostName', 'nixos-check');
await setInput('control-services-applyTarget', 'after');
await sleep(600);

await clickText('Diff');
await sleep(600);
const diffText = await evaluate('document.querySelector("pre.diff")?.textContent ?? ""');
if (!diffText.includes('nixos-check')) throw new Error('diff did not show the staged hostname');

const mediaItem = await evaluate(`(() => {
  const item = [...document.querySelectorAll('.media-item')]
    .find((candidate) => candidate.querySelector('.media-name')?.textContent.trim() === 'one.png');
  if (!item) return false;
  item.click();
  return true;
})()`);
if (!mediaItem) throw new Error('media picker did not list one.png');
await sleep(400);
const background = await evaluate('document.getElementById("control-services-background")?.value ?? ""');
if (!background.includes('one.png')) throw new Error(`media selection did not change the value: ${background}`);

await clickText('Save');
await sleep(2500);
const savedStatus = await evaluate('document.querySelector(".bottombar .left")?.textContent ?? ""');
if (!savedStatus.includes('Saved')) throw new Error(`save did not finish: ${savedStatus}`);
if (fixture) {
  const saved = readFileSync(`${fixture}/modules/system.nix`, 'utf8');
  if (!saved.includes('nixos-check')) throw new Error('save did not write the hostname');
  if (!existsSync(`${fixture}/applied.txt`)) throw new Error('the apply command did not run');
}

await clickText('Add option');
await sleep(800);
await evaluate(`(() => {
  const panel = [...document.querySelectorAll('.panel')]
    .find((item) => item.querySelector('h2')?.textContent.trim() === 'Add option');
  if (!panel) throw new Error('add option panel missing');
  const inputs = panel.querySelectorAll('input');
  const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set;
  setter.call(inputs[0], 'services.added');
  inputs[0].dispatchEvent(new Event('input', { bubbles: true }));
  setter.call(inputs[1], 'Added option');
  inputs[1].dispatchEvent(new Event('input', { bubbles: true }));
  setter.call(inputs[2], '"added"');
  inputs[2].dispatchEvent(new Event('input', { bubbles: true }));
  panel.querySelector('button[type=submit]').click();
})()`);
await sleep(1500);
const addedBody = await evaluate('document.body.innerText');
if (!addedBody.includes('Added option')) throw new Error('add option did not create a control');

await clickText('Add file');
await sleep(800);
if (fixture) {
  await evaluate(`(() => {
    const panel = [...document.querySelectorAll('.panel')]
      .find((item) => item.querySelector('h2')?.textContent.trim() === 'Add file');
    if (!panel) throw new Error('add file panel missing');
    const inputs = panel.querySelectorAll('input');
    const setter = Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set;
    setter.call(inputs[0], ${JSON.stringify(`${fixture}/modules/extra.nix`)});
    inputs[0].dispatchEvent(new Event('input', { bubbles: true }));
    setter.call(inputs[1], 'flake.modules.nixos.extra');
    inputs[1].dispatchEvent(new Event('input', { bubbles: true }));
    panel.querySelector('button[type=submit]').click();
  })()`);
  await sleep(1500);
  if (!existsSync(`${fixture}/modules/extra.nix`)) throw new Error('add file did not create the module');
}

await setInput('control-networking-hostName', 'nixos-rebuilt');
await sleep(400);
await clickText('Save & rebuild');
await sleep(3000);
const rebuildPanel = await evaluate(`(() => {
  const panel = [...document.querySelectorAll('.panel')]
    .find((item) => item.querySelector('h2')?.textContent.trim() === 'Rebuild output');
  return panel?.querySelector('pre')?.textContent ?? '';
})()`);
if (!rebuildPanel.includes('rebuilt')) throw new Error(`rebuild output missing: ${rebuildPanel}`);

await clickText('Flake Inputs');
await sleep(600);
const flakeBody = await evaluate('document.body.innerText');
if (!flakeBody.includes('Add input')) throw new Error('flake inputs view did not open');

if (errors.length) throw new Error(errors.join('\n'));
console.log(JSON.stringify({ cards, csvFields, monitorTiles, gridHandles, dragged: true, gridDragged: true, add: true, rename: true, remove: true, diff: true, save: true, media: true, apply: true, addOption: true, addFile: true, rebuild: true, flakeInputs: true }));
socket.close();
