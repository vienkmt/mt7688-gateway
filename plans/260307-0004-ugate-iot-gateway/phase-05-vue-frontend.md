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

**DashboardView.vue (Status Page - default after login):**
```vue
<template>
  <div class="status-page">
    <!-- System State -->
    <section class="system-state">
      <h3>System State</h3>
      <div class="grid">
        <div><label>Product</label><span>ugate</span></div>
        <div><label>MAC</label><span>{{ status.mac }}</span></div>
        <div><label>IP</label><span>{{ status.ip }}</span></div>
        <div><label>Gateway</label><span>{{ status.gateway }}</span></div>
        <div><label>Firmware</label><span>{{ status.version }}</span></div>
        <div><label>Uptime</label><span>{{ status.uptime }}</span></div>
        <div><label>CPU</label><span>{{ status.cpu }}%</span></div>
        <div><label>RAM</label><span>{{ status.ram_used }}/{{ status.ram_total }} MB</span></div>
        <div><label>Flash</label><span>{{ status.flash_used }}/{{ status.flash_total }} MB</span></div>
        <div><label>WiFi</label><span>{{ status.wifi_state }} ({{ status.wifi_rssi }} dBm)</span></div>
      </div>
    </section>

    <!-- Serial Port State -->
    <section class="serial-state">
      <h3>Serial Port State</h3>
      <div class="grid">
        <div><label>Received Bytes</label><span>{{ status.uart.rx_bytes }}</span></div>
        <div><label>Received Frames</label><span>{{ status.uart.rx_frames }}</span></div>
        <div><label>Sent Bytes</label><span>{{ status.uart.tx_bytes }}</span></div>
        <div><label>Sent Frames</label><span>{{ status.uart.tx_frames }}</span></div>
        <div><label>Failed</label><span>{{ status.uart.failed }}</span></div>
        <div><label>Config</label><span>{{ status.uart.config }}</span></div>
      </div>
    </section>

    <!-- Channel States -->
    <section v-if="status.mqtt.enabled" class="channel-state">
      <h3>MQTT State</h3>
      <div class="grid">
        <div><label>State</label><span>{{ status.mqtt.state }}</span></div>
        <div><label>Published</label><span>{{ status.mqtt.published }}</span></div>
        <div><label>Failed</label><span>{{ status.mqtt.failed }}</span></div>
      </div>
    </section>

    <section v-if="status.tcp.enabled" class="channel-state">
      <h3>TCP State</h3>
      <div class="grid">
        <div><label>Mode</label><span>{{ status.tcp.mode }}</span></div>
        <div><label>State</label><span>{{ status.tcp.state }}</span></div>
        <div><label>Connections</label><span>{{ status.tcp.connections }}</span></div>
      </div>
    </section>

    <!-- GPIO Controls -->
    <section class="gpio-controls">
      <h3>GPIO Outputs</h3>
      <div class="buttons">
        <GpioButton v-for="i in 4" :key="i" :pin="i" :state="status.gpio[i-1]" @toggle="toggleGpio" />
      </div>
    </section>
  </div>
</template>

<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue'
import { useWebSocket } from '@/composables/useWebSocket'

const { status, connect, sendGpio } = useWebSocket()
// WebSocket sends status every 1 second

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
      <label><input type="checkbox" v-model="config.mqtt.tls"> TLS</label>
      <input v-model="config.mqtt.topic" placeholder="Topic">
      <input v-model="config.mqtt.client_id" placeholder="Client ID">
      <label>Publish QoS:
        <select v-model.number="config.mqtt.qos">
          <option :value="0">0 - At most once</option>
          <option :value="1">1 - At least once</option>
          <option :value="2">2 - Exactly once</option>
        </select>
      </label>
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

    <section>
      <h3>UART / Serial</h3>
      <small>Device: /dev/ttyS1 (hardcoded)</small>
      <input v-model.number="config.uart.baudrate" type="number" placeholder="Baudrate">
      <input v-model.number="config.uart.buffer_size" type="number" placeholder="Buffer Size">

      <h4>Protocol Settings</h4>
      <select v-model="config.uart.protocol">
        <option value="none">None (Gap-based)</option>
        <option value="frame">Frame (Fixed-length)</option>
        <option value="modbus">Modbus RTU</option>
      </select>

      <!-- Protocol = None -->
      <div v-if="config.uart.protocol === 'none'">
        <label>Gap (ms): <input v-model.number="config.uart.gap_ms" type="number"></label>
      </div>

      <!-- Protocol = Frame -->
      <div v-if="config.uart.protocol === 'frame'">
        <label>Frame Length: <input v-model.number="config.uart.frame_length" type="number"></label>
        <label>Frame Timeout (ms): <input v-model.number="config.uart.frame_timeout" type="number"></label>
        <label><input type="checkbox" v-model="config.uart.tag_enabled"> Enable Tags</label>
        <div v-if="config.uart.tag_enabled">
          <label>Tag Head: <input v-model="config.uart.tag_head" placeholder="0x02"></label>
          <label>Tag Tail: <input v-model="config.uart.tag_tail" placeholder="0x03"></label>
        </div>
      </div>

      <!-- Protocol = Modbus -->
      <div v-if="config.uart.protocol === 'modbus'">
        <label>Slave Address: <input v-model="config.uart.slave_addr" placeholder="0x01"></label>
        <small>Gap auto-calculated from baudrate (3.5T)</small>
      </div>
    </section>

    <section>
      <h3>Security</h3>
      <input v-model="newPassword" type="password" placeholder="New Password">
      <button type="button" @click="changePassword">Change Password</button>
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

const newPassword = ref('')

function save() {
  store.save(config.value)
}

async function changePassword() {
  if (!newPassword.value) return
  await fetch('/api/password', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ password: newPassword.value }),
  })
  newPassword.value = ''
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
