# Phase 5: Vue.js Frontend

**Priority:** Medium
**Status:** pending
**Effort:** 3 days
**Depends on:** Phase 3 (Web Server)

## Context

Vue.js SPA embedded trong binary. Real-time monitoring + control via WebSocket.

## Project Structure

```
ugate/frontend/
├── package.json
├── vite.config.ts
├── index.html
├── src/
│   ├── main.ts
│   ├── App.vue
│   ├── router/
│   │   └── index.ts
│   ├── stores/
│   │   ├── auth.ts
│   │   ├── config.ts
│   │   └── ws.ts
│   ├── composables/
│   │   └── useWebSocket.ts
│   ├── views/
│   │   ├── LoginView.vue
│   │   ├── DashboardView.vue
│   │   ├── ConfigView.vue
│   │   └── GpioView.vue
│   └── components/
│       ├── StatusCard.vue
│       ├── DataStream.vue
│       └── GpioButton.vue
└── dist/              # Build output
```

## Implementation Steps

### 1. Initialize Vue project

```bash
cd ugate
npm create vite@latest frontend -- --template vue-ts
cd frontend
npm install vue-router pinia
```

### 2. Create WebSocket composable

```typescript
// src/composables/useWebSocket.ts
import { ref, onUnmounted } from 'vue'

export function useWebSocket() {
  const data = ref<string[]>([])
  const connected = ref(false)
  let ws: WebSocket | null = null

  function connect() {
    ws = new WebSocket(`ws://${location.host}/ws`)

    ws.onopen = () => { connected.value = true }
    ws.onclose = () => {
      connected.value = false
      setTimeout(connect, 3000) // Reconnect
    }
    ws.onmessage = (e) => {
      data.value.push(e.data)
      if (data.value.length > 100) data.value.shift()
    }
  }

  function send(cmd: object) {
    if (ws?.readyState === WebSocket.OPEN) {
      ws.send(JSON.stringify(cmd))
    }
  }

  function sendGpio(pin: number, state: string) {
    send({ cmd: 'gpio', pin, state })
  }

  function sendUart(data: string) {
    send({ cmd: 'uart', data })
  }

  onUnmounted(() => ws?.close())

  return { data, connected, connect, sendGpio, sendUart }
}
```

### 3. Create stores

```typescript
// src/stores/auth.ts
import { defineStore } from 'pinia'
import { ref } from 'vue'

export const useAuthStore = defineStore('auth', () => {
  const loggedIn = ref(false)

  async function login(password: string) {
    const res = await fetch('/api/login', {
      method: 'POST',
      body: JSON.stringify({ password }),
    })
    loggedIn.value = res.ok
    return res.ok
  }

  return { loggedIn, login }
})
```

```typescript
// src/stores/config.ts
import { defineStore } from 'pinia'
import { ref } from 'vue'

export const useConfigStore = defineStore('config', () => {
  const config = ref<Config | null>(null)

  async function load() {
    const res = await fetch('/api/config')
    config.value = await res.json()
  }

  async function save(newConfig: Config) {
    await fetch('/api/config', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(newConfig),
    })
    config.value = newConfig
  }

  return { config, load, save }
})
```

### 4. Create views

**DashboardView.vue:**
```vue
<template>
  <div class="dashboard">
    <StatusCard title="UART" :connected="wsConnected" />
    <StatusCard title="MQTT" :connected="mqttStatus" />

    <div class="data-stream">
      <h3>Live Data</h3>
      <div v-for="(line, i) in data" :key="i">{{ line }}</div>
    </div>

    <div class="gpio-controls">
      <GpioButton v-for="i in 4" :key="i" :pin="i-1" @toggle="toggleGpio" />
    </div>
  </div>
</template>

<script setup lang="ts">
import { onMounted } from 'vue'
import { useWebSocket } from '@/composables/useWebSocket'

const { data, connected: wsConnected, connect, sendGpio } = useWebSocket()

function toggleGpio(pin: number) {
  sendGpio(pin, 'toggle')
}

onMounted(connect)
</script>
```

**ConfigView.vue:**
```vue
<template>
  <form @submit.prevent="save">
    <section>
      <h3>MQTT</h3>
      <label><input type="checkbox" v-model="config.mqtt.enabled"> Enabled</label>
      <input v-model="config.mqtt.broker" placeholder="Broker">
      <input v-model.number="config.mqtt.port" type="number" placeholder="Port">
    </section>

    <section>
      <h3>HTTP</h3>
      <label><input type="checkbox" v-model="config.http.enabled"> Enabled</label>
      <input v-model="config.http.url" placeholder="URL">
    </section>

    <section>
      <h3>TCP</h3>
      <label><input type="checkbox" v-model="config.tcp.enabled"> Enabled</label>
      <select v-model="config.tcp.mode">
        <option value="server">Server</option>
        <option value="client">Client</option>
        <option value="both">Both</option>
      </select>
    </section>

    <button type="submit">Save</button>
  </form>
</template>

<script setup lang="ts">
import { onMounted } from 'vue'
import { useConfigStore } from '@/stores/config'

const store = useConfigStore()
const config = computed(() => store.config)

onMounted(() => store.load())

function save() {
  store.save(config.value)
}
</script>
```

### 5. Build configuration

```typescript
// vite.config.ts
import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'

export default defineConfig({
  plugins: [vue()],
  build: {
    outDir: 'dist',
    assetsInlineLimit: 100000, // Inline small assets
    rollupOptions: {
      output: {
        manualChunks: undefined, // Single bundle
      },
    },
  },
})
```

### 6. Embed in Rust

```rust
// ugate/build.rs
use std::process::Command;

fn main() {
    // Build Vue.js
    let status = Command::new("npm")
        .args(["run", "build"])
        .current_dir("frontend")
        .status()
        .expect("Failed to build frontend");

    if !status.success() {
        panic!("Frontend build failed");
    }

    println!("cargo:rerun-if-changed=frontend/src");
}
```

```rust
// ugate/src/web/static_files.rs
static INDEX_HTML: &[u8] = include_bytes!("../../frontend/dist/index.html");
static JS_BUNDLE: &[u8] = include_bytes!("../../frontend/dist/assets/index.js");
static CSS_BUNDLE: &[u8] = include_bytes!("../../frontend/dist/assets/index.css");
```

## Files to Create

| File | Description |
|------|-------------|
| frontend/package.json | Vue project config |
| frontend/vite.config.ts | Build config |
| frontend/src/main.ts | Entry point |
| frontend/src/App.vue | Root component |
| frontend/src/router/index.ts | Router |
| frontend/src/stores/*.ts | Pinia stores |
| frontend/src/composables/useWebSocket.ts | WS hook |
| frontend/src/views/*.vue | Page views |
| frontend/src/components/*.vue | UI components |
| ugate/build.rs | Build script |

## Todo

- [ ] Initialize Vue project
- [ ] Setup router
- [ ] Setup Pinia stores
- [ ] Create WebSocket composable
- [ ] Create LoginView
- [ ] Create DashboardView
- [ ] Create ConfigView
- [ ] Create GpioView
- [ ] Create UI components
- [ ] Configure Vite for minimal bundle
- [ ] Create Rust build.rs
- [ ] Test embedded serve
- [ ] Optimize bundle size (<300KB)

## Success Criteria

- [ ] Login page works
- [ ] Dashboard shows real-time data
- [ ] Config form saves
- [ ] GPIO buttons toggle
- [ ] WebSocket reconnects on disconnect
- [ ] Bundle size <300KB gzipped

## Next Phase

Phase 6: Integration & Testing
