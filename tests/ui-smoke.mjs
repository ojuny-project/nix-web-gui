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
const base = process.env.NWG_BASE ?? 'http://127.0.0.1:8899';
const cdp = process.env.NWG_CDP ?? 'http://127.0.0.1:9223';
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
    errors.push(message.params.exceptionDetails.text ?? 'exception');
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
  if (result.result?.exceptionDetails) throw new Error(result.result.exceptionDetails.text);
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
for (const item of ['Users', 'alice', 'Add row', 'Binds', 'reload', 'Bar', 'clock', 'Monitors', 'eDP-1', 'Hostname']) {
  if (!body.includes(item)) {
    console.error(body.slice(0, 1500));
    throw new Error(`missing text: ${item}`);
  }
}
const cards = await evaluate('document.querySelectorAll(".row-card").length');
const csvFields = await evaluate('document.querySelectorAll("input.csv-input").length');
const monitorTiles = await evaluate('document.querySelectorAll(".monitor-tile").length');
if (cards < 4) throw new Error(`expected card rows, saw ${cards}`);
if (monitorTiles < 1) throw new Error('expected a monitor canvas tile');
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
if (errors.length) throw new Error(errors.join('\n'));
console.log(JSON.stringify({ cards, csvFields, monitorTiles, dragged: true, add: true, rename: true, remove: true }));
socket.close();
