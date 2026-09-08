// Regression checks against actual frontend source.
const assert = require('node:assert/strict');
const fs = require('node:fs');
const vm = require('node:vm');
const ts = require('typescript');
const path = require('node:path');

function harness(file, dependencies = {}) {
  const slots = [], effects = [], timers = new Map();
  let cursor = 0, timerId = 0;
  const react = {
    useState(initial) {
      const i = cursor++;
      if (!(i in slots)) slots[i] = initial;
      return [slots[i], value => { slots[i] = typeof value === 'function' ? value(slots[i]) : value; }];
    },
    useRef(initial) {
      const i = cursor++;
      if (!(i in slots)) slots[i] = { current: initial };
      return slots[i];
    },
    useCallback: fn => fn,
    useEffect: fn => effects.push(fn),
    createElement: (type, props, ...children) => ({ type, props: { ...props, children } }),
  };
  const exports = {};
  const context = {
    exports, React: react, console: { ...console, error() {} },
    require: name => name === 'react' ? react : dependencies[name] || {},
    setTimeout: fn => { timers.set(++timerId, fn); return timerId; },
    clearTimeout: id => timers.delete(id),
  };
  const code = ts.transpileModule(fs.readFileSync(path.join(__dirname, '..', file), 'utf8'), {
    compilerOptions: { module: ts.ModuleKind.CommonJS, jsx: ts.JsxEmit.React, esModuleInterop: true },
  }).outputText;
  vm.runInNewContext(code, context, { filename: file });
  return { exports, slots, effects, timers, render(fn) { cursor = 0; return fn(); } };
}
const deferred = () => {
  let resolve, reject;
  const promise = new Promise((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
};
function descendants(node) {
  if (!node || typeof node !== 'object') return [];
  if (Array.isArray(node)) return node.flatMap(descendants);
  return [node, ...descendants(node.props?.children)];
}

(async () => {
  const connect = deferred();
  const api = {
    connect: () => connect.promise,
    cancelConnection: async () => {},
    getConnectionState: async () => 'DISCONNECTED',
  };
  const store = harness('src/stores/useAppStore.ts', { '../services/api': { api } });
  const first = store.render(() => store.exports.useAppStore());
  const connecting = first.triggerConnect();
  await first.triggerCancel();
  assert.equal(store.render(() => store.exports.useAppStore()).connectionState, 'DISCONNECTED');
  connect.reject('Connection cancelled by user');
  await connecting;
  assert.equal(store.render(() => store.exports.useAppStore()).connectionState, 'DISCONNECTED');
  console.log('PASS: late connect rejection cannot overwrite successful cancellation');

  const health = deferred();
  const polling = harness('src/stores/useAppStore.ts', {
    '../services/api': { api: {
      getSettings: async () => ({}), getHealthStatus: () => health.promise,
      getLogs: async () => [], getConnectionState: async () => 'DISCONNECTED',
    } },
    '@tauri-apps/api/event': { listen: async () => () => {} },
  });
  polling.render(() => polling.exports.useAppStore());
  const cleanup = polling.effects[0]();
  await Promise.resolve();
  const [id, poll] = [...polling.timers][0];
  polling.timers.delete(id);
  const pendingPoll = poll();
  cleanup();
  assert.equal(polling.timers.size, 0);
  health.resolve({});
  await pendingPoll;
  assert.equal(polling.timers.size, 0);
  console.log('PASS: in-flight poll cannot schedule another timer after effect cleanup');

  const settings = { general: {}, applicationRules: [], aether: {}, secondaryProxy: {}, singBox: {}, compatibility: {} };
  const save = deferred();
  const component = harness('src/features/settings/SettingsView.tsx');
  const props = { settings, onSave: () => save.promise, onReset() {} };
  const tree = component.render(() => component.exports.SettingsView(props));
  const apply = descendants(tree).find(n => n.type === 'button' && n.props.onClick?.name === 'handleSave');
  assert.ok(apply);
  const saving = apply.props.onClick();
  assert.equal(component.slots[5], false);
  assert.equal(component.slots[6], true);
  save.resolve();
  await saving;
  assert.equal(component.slots[5], true);
  assert.equal(component.slots[6], false);
  console.log('PASS: APPLIED indicator waits for save completion');
  const newSettings = { ...settings, general: { autoConnect: true } };
  component.render(() => component.exports.SettingsView({ ...props, settings: newSettings }));
  component.effects.at(-1)();
  assert.equal(component.slots[0], newSettings);
  console.log('PASS: SettingsView accepts replacement settings');

  let resetCalls = 0;
  const app = harness('src/App.tsx', {
    './stores/useAppStore': { useAppStore: () => ({ settings: { ...settings, firstRunCompleted: true }, resetSettings: async () => { resetCalls++; return settings; } }) },
    './features/settings/SettingsView': { SettingsView: 'SettingsView' },
  });
  app.slots[0] = 'settings';
  const appTree = app.render(() => app.exports.App());
  await descendants(appTree).find(n => n.type === 'SettingsView').props.onReset();
  assert.equal(resetCalls, 1);
  console.log('PASS: Reset invokes backend reset operation');
})().catch(error => { console.error(error); process.exitCode = 1; });
