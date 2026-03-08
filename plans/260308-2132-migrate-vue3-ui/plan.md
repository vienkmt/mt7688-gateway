# Plan: Migrate UI from Vanilla JS to Vue 3

## Context
- Current: 10 vanilla JS files (~69KB) using `h()` hyperscript pattern
- Problem: Code khó đọc, khó maintain, không quen thuộc với dev viết HTML/CSS thuần
- Goal: Rewrite UI dùng Vue 3 full (with template compiler) để code clean như viết HTML thường

## Approach
- Embed `vue.global.prod.min.js` (~40KB) vào build.rs concat, serve cùng HTML
- Viết Vue components dạng inline template trong JS (không cần .vue files, không cần bundler)
- Migrate từng page một, giữ cả 2 hệ thống chạy song song trong quá trình migrate

## Architecture

```
ugate/src/
  js/
    00-vue.min.js          # Vue 3 global production build (~40KB)
    01-core.js             # Vue app setup, global state (reactive), router logic
    02-components.js       # Shared components: AppHeader, NavBar, Toast, PendingBanner...
    03-page-status.js      # StatusPage component
    04-page-config.js      # ConfigPage component
    05-page-uart.js        # UartPage component
    06-page-network.js     # NetworkPage component
    07-page-routing.js     # RoutingPage component
    08-page-toolbox.js     # ToolboxPage component
    09-page-system.js      # SystemPage component
    10-app.js              # Mount Vue app, WebSocket handler
  index-template.html      # HTML with Vue template markup + {{JS_BUNDLE}}
```

## Key Patterns

### State Management
```js
// Before (vanilla):
const S = { page: 'login', connected: false, ... };
function render() { /* rebuild DOM */ }

// After (Vue 3 reactive):
const store = Vue.reactive({ page: 'login', connected: false, ... });
// Auto re-render khi state thay đổi
```

### Component Example
```js
// Before:
function renderConfig() {
  return h('div', { cls: 'card' },
    h('h3', {}, 'General'),
    h('div', { cls: 'cf' },
      lbl('Tên thiết bị'), inp('text', c.general, 'device_name')
    )
  );
}

// After (Vue 3 inline template):
app.component('config-page', {
  template: `
    <div class="card">
      <h3>General</h3>
      <div class="cf">
        <span class="lbl">Tên thiết bị</span>
        <input type="text" v-model="store.config.general.device_name">
      </div>
    </div>
  `,
  setup() { return { store } }
});
```

### HTML Template
```html
<div id="app">
  <template v-if="store.page === 'login'">
    <login-page/>
  </template>
  <template v-else>
    <app-header/>
    <nav-bar/>
    <component :is="store.page + '-page'"/>
  </template>
</div>
```

## Migration Phases

### Phase 1: Setup Vue + Core (01-core, 02-components, 10-app)
- Download vue.global.prod.min.js, add to js/00-vue.min.js
- Create Vue app with reactive store
- Migrate: LoginPage, AppHeader, NavBar, Toast
- Verify: login flow works

### Phase 2: Status + Config pages (03, 04)
- Migrate renderStatus → StatusPage component
- Migrate renderConfig → ConfigPage component
- Shared: pbar, chBadge, lbl components

### Phase 3: UART page (05)
- Migrate renderUart, renderData → UartPage
- Handle incremental stream rendering with Vue

### Phase 4: Network page (06)
- Largest page (~570 LOC formatted)
- Migrate WiFi, LAN/WAN, NTP sections
- WiFi scan modal

### Phase 5: Routing + Toolbox + System (07, 08, 09)
- Migrate remaining pages
- WebSocket integration in 10-app.js

### Phase 6: Cleanup
- Remove old h() helper
- Optimize template sizes
- Test all 7 tabs + WebSocket + login on device

## Risks
| Risk | Mitigation |
|------|-----------|
| Vue 40KB increases binary | Still fits in 16MB flash, total ~110KB vs 69KB |
| Template compiler slower | Negligible on modern browsers, even embedded |
| Rewrite introduces bugs | Migrate page-by-page, test each phase |
| WebSocket integration | Vue reactivity auto-updates, simpler than manual DOM |

## Benefits
- Code reads like HTML — familiar to HTML/CSS devs
- v-model, v-if, v-for eliminates manual DOM manipulation
- Reactive state — no more manual render() calls
- Components = reusable, testable units
- ~30% less code for same UI

## Status
- [x] Phase 1: Setup Vue + Core (00-vue.min.js, 01-core.js, 02-components.js, 10-app.js)
- [x] Phase 2: Status + Config (03-page-status.js, 04-page-config.js)
- [x] Phase 3: UART (05-page-uart.js)
- [x] Phase 4: Network (06-page-network.js)
- [x] Phase 5: Routing + Toolbox + System (07, 08, 09)
- [x] Phase 6: Cleanup — old h()/helpers removed, build.rs updated
- [ ] Deploy test on device
