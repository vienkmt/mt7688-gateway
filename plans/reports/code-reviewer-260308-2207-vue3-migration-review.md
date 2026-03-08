# Vue 3 UI Migration Review

**Files:** 01-core.js through 10-app.js, modals-loader.js, index-template.html
**LOC:** ~880 across 12 files
**Focus:** Bug hunting in Vue 3 migration

## Overall Assessment

Migration is clean overall. No stale `S.` or `h()` references found. Component registrations are complete. A few real bugs and several medium-priority issues follow.

---

## CRITICAL Issues

### 1. `keep-alive` with dynamic component requires `key` -- currently broken

**File:** `10-app.js:49-51`
```html
<keep-alive>
  <component :is="store.page + '-page'"/>
</keep-alive>
```

**Problem:** `<keep-alive>` expects its child to have a unique `key` to properly cache/restore instances. Without it, Vue may not correctly distinguish between different page components, causing stale state or rendering errors when switching between pages that have similar structures.

More critically: `<keep-alive>` caches component instances including their local `Vue.ref()` state. When navigating between pages, forms retain old input values (e.g., WiFi passwords, NTP servers). This is likely **unintended** for a config UI where users expect fresh state from the server on each visit.

**Fix:** Either add `:key="store.page"` or remove `<keep-alive>` entirely if fresh-load-on-navigate is desired:
```html
<component :is="store.page + '-page'" :key="store.page"/>
```

### 2. Dynamic component name mismatch -- `config` page won't render

**File:** `10-app.js:50`
```
store.page + '-page'  =>  "config-page"
```
But the NavBar tab ID is `'config'` (line 152, 02-components.js), and the component is registered as `'config-page'` (line 74, 10-app.js). The **component definition** however is `ConfigPage` (04-page-channels.js). This is actually **fine** -- it resolves correctly.

BUT: check the login flow. After login, `store.page = 'status'` resolves to `status-page` -- correct. All tab IDs map: `status`, `config`, `uart`, `network`, `routing`, `toolbox`, `system` -> `*-page`. All registered. **No mismatch found.**

---

## HIGH Priority

### 3. `showDataWrapHelp()` called from template but defined in modals-loader.js loaded AFTER main bundle

**File:** `04-page-channels.js:9`, `index-template.html:15-16`

```html
<!-- index-template.html -->
<script>{{JS_BUNDLE}}</script>      <!-- 01-core through 10-app -->
<script src="/modals.js"></script>   <!-- modals-loader.js -->
```

The `@click.prevent="showDataWrapHelp()"` in ConfigPage template calls a function defined in `modals-loader.js`. Since modals.js loads after the bundle, the function IS available at runtime (templates execute lazily, not at parse time). **This works but is fragile** -- any eager evaluation path (e.g., SSR or template compilation at mount) would fail.

**Verdict:** Works in practice. Low-risk but worth noting.

### 4. `cfg` computed in UartPage returns empty object when config is null -- v-model binds will fail silently

**File:** `05-page-uart.js:95`
```js
const cfg = Vue.computed(() => store.config ? store.config.uart : {});
```

When `store.config` is null, `cfg` is `{}`. The template uses `v-model="cfg.baudrate"` etc. Vue v-model on a computed getter-only property with no setter will **silently fail** -- user changes to the dropdowns won't be saved because you're mutating properties on a plain `{}` object that isn't connected to the store.

After `loadConfig()` completes and `store.config` is populated, a new `cfg` computed value is returned, and the old `{}` is discarded. So the **real issue** is: the `cfg` computed returns a **read-only computed ref**, but the template uses `v-model` which writes to `cfg.baudrate`. This actually works because Vue computed returns the inner object reference, and `v-model` mutates the property on that object directly (not the computed itself). So `cfg.baudrate = x` mutates `store.config.uart.baudrate`. **This is fine for the happy path.**

**Actual bug:** If user interacts with UART dropdowns BEFORE config loads (the `v-if="store._uartOpen && store.config"` guard prevents this). **Not a bug in practice.**

### 5. MQTT QoS `select` uses string values but backend likely expects number

**File:** `04-page-channels.js:52-54`
```html
<select v-model="c.mqtt.qos">
  <option value="0">0</option><option value="1">1</option><option value="2">2</option>
</select>
```

Without `v-model.number`, the selected value is a string `"0"`, `"1"`, `"2"`. When sent to backend via `JSON.stringify(store.config)`, it serializes as `"qos":"1"` instead of `"qos":1`. If backend expects integer, this is a **data type bug**.

**Fix:** Use `v-model.number="c.mqtt.qos"` or add `:value="0"` (number binding).

### 6. WiFi scan modal uses innerHTML with user-controlled SSID -- XSS vulnerability

**File:** `06-page-network.js:160`
```js
left.innerHTML = '<div style="...">' + n.ssid + '</div>...';
```

If an attacker broadcasts a WiFi network with SSID containing `<script>` or `<img onerror=...>`, this will execute arbitrary JavaScript. The SSID comes from a scan API response which reflects real WiFi names.

**Fix:** Use `textContent` or create elements properly:
```js
const nameDiv = document.createElement('div');
nameDiv.textContent = n.ssid;
```

### 7. `window` object exposed in setup return -- works but non-standard

**File:** `09-page-system.js:112`
```js
return { store, v, ui, ipkName, restoreName, inputSt, window };
```

Template uses `window.location.href`. This works because `window` is a global, but returning it from `setup()` is cleaner. **No bug, just unusual.**

---

## MEDIUM Priority

### 8. Login `doLogin()` has no try/catch -- network error crashes silently

**File:** `10-app.js:24-36`
```js
async doLogin() {
  const r = await fetch('/api/login', { ... });  // no try/catch
```

If the device is offline, this throws an unhandled promise rejection.

**Fix:** Wrap in try/catch, show `store.loginErr = 'Lỗi kết nối'`.

### 9. UART `sendTx` bypasses Vue ref system by directly manipulating DOM input

**File:** `05-page-uart.js:119-127`
```js
const input = this.$refs.txInput;
const v = input.value.trim();
// ...
input.value = '';
```

The input is not bound with `v-model` -- it uses a raw DOM ref. This works but the value is invisible to Vue's reactivity. If any other code needs to read the TX input value reactively, it won't work. Minor issue since no other code reads it.

### 10. `NtpSection` watches `store.ntp` but only fires on reference change, not deep mutation

**File:** `06-page-network.js:324`
```js
Vue.watch(() => store.ntp, (n) => { ... }, { immediate: true });
```

If `store.ntp` is mutated in-place (e.g., `store.ntp.enabled = false`), the watcher won't fire because the reference hasn't changed. However, `store.ntp` is always set as a whole object from API (`store.ntp = await r.json()`), so **this is fine in practice**.

### 11. `LanWanSection` same shallow watch pattern

**File:** `06-page-network.js:248`
Same analysis as #10. Works because `store.net` is always replaced wholesale.

### 12. Toolbox double-initialization of `Vue.reactive` arrays

**File:** `08-page-toolbox.js:42`
```js
store.toolbox = Vue.reactive({ tool: 'ping', target: '', lines: Vue.reactive([]), running: false });
```

`store.toolbox` is already inside `Vue.reactive(store)`, so `lines: Vue.reactive([])` is **double-wrapping** in reactive. Vue handles this gracefully (returns the same proxy), but it's unnecessary and confusing. Same pattern at `08-page-toolbox.js:108` for syslog.

Same double-init exists in WebSocket handler at `10-app.js:106` and `10-app.js:114`.

### 13. `store.newRoute` object replacement breaks reactivity binding

**File:** `07-page-routing.js:133`
```js
store.newRoute = { name: '', target: '', ... };
```

After a route is added, the entire `newRoute` object is replaced. Since `store` is `Vue.reactive`, the new plain object gets auto-wrapped. Template bindings (`v-model="store.newRoute.name"`) will still work because they access via `store.newRoute` each time. **Not a bug.**

---

## LOW Priority

### 14. UART baudrate options are strings, not numbers

**File:** `05-page-uart.js:94`
```js
const bauds = ['9600', '19200', ...];
```

`v-model="cfg.baudrate"` with string options means the selected baudrate is stored as string. If backend expects integer, similar issue to #5.

### 15. No error feedback when `loadConfig()`, `loadNetwork()`, etc. fail

Multiple files silently swallow errors with `catch (_) {}`. The UI shows "Dang tai..." forever if API calls fail. Consider adding timeout or error state.

### 16. `tbRun()` missing Content-Type header

**File:** `08-page-toolbox.js:64-66`
```js
const r = await fetch('/api/toolbox/run', {
  method: 'POST',
  body: JSON.stringify({ tool: tb.tool, target: tb.target })
});
```

Missing `headers: { 'Content-Type': 'application/json' }`. Backend may reject or misparse the body.

---

## Component Registration Audit

All components defined are registered in `10-app.js:57-85`. Cross-referenced:

| Component | Defined | Registered | Used In |
|-----------|---------|------------|---------|
| signal-bars | 02 | yes | 03, 06 |
| iface-badge | 02 | yes | 07 |
| ch-badge | 02 | yes | 03 |
| progress-bar | 02 | yes | 03 |
| switch-toggle | 02 | yes | 02, 06 |
| channel-header | 02 | yes | 04 |
| pending-banner | 02 | yes | 06, 07 |
| pwd-input | 06 | yes | 06 |
| app-header | 02 | yes | 10 |
| nav-bar | 02 | yes | 10 |
| login-page | 10 | yes | 10 |
| All page components | 03-09 | yes | 10 (dynamic) |
| wifi-section | 06 | yes | 06 |
| lan-wan-section | 06 | yes | 06 |
| ntp-section | 06 | yes | 06 |
| metrics-section | 07 | yes | 07 |
| routes-section | 07 | yes | 07 |
| syslog-section | 08 | yes | 08 |

**No missing registrations found.**

---

## Stale Reference Audit

- `S.` (old global state): **None found** in JS files (only in Vue minified code)
- `h()` (old hyperscript): **None found**
- `render()` (old manual render): **None found**
- `showDataWrapHelp()`: Defined in modals-loader.js, called from 04-page-channels.js -- works due to load order

---

## Recommended Actions (Prioritized)

1. **[CRITICAL]** Remove `<keep-alive>` or add `:key` -- stale form state across page navigations
2. **[HIGH]** Fix XSS in WiFi scan modal -- use `textContent` instead of `innerHTML` for SSID
3. **[HIGH]** Add `v-model.number` to MQTT QoS select (and verify baudrate handling)
4. **[HIGH]** Add `Content-Type: application/json` header to toolbox run fetch
5. **[MEDIUM]** Add try/catch to `doLogin()` for network error handling
6. **[LOW]** Remove double `Vue.reactive()` wrapping for toolbox/syslog lines arrays

## Unresolved Questions

- Is `<keep-alive>` intentional for performance, or accidental from migration? If intentional, `onActivated()` hooks should reload fresh data.
- Does the Rust backend tolerate string QoS values or strictly require integers?
- Is the UART baudrate stored as string or number in UCI config?
