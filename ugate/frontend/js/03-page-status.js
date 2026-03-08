// --- Status page Vue component ---

const StatusPage = {
  template: `
    <div>
      <div class="card">
        <h3>Hệ thống</h3>
        <div class="sys-info cf" style="grid-template-columns:auto 1fr auto 1fr auto 1fr">
          <span class="lbl">Phiên bản</span>
          <span style="color:#e2e8f0;font-weight:500">{{ s.version || '-' }}</span>
          <span class="lbl">Uptime</span>
          <span style="color:#e2e8f0;font-weight:500">{{ s.uptime || '-' }}</span>
          <span class="lbl">Thời gian</span>
          <span style="color:#e2e8f0;font-weight:500">{{ s.datetime || '-' }}</span>
        </div>
        <progress-bar label="CPU" :val="cpu" :max="100"
          :color="cpu > 80 ? '#ef4444' : cpu > 50 ? '#f59e0b' : '#22c55e'"
          :text="cpu + '%'"/>
        <progress-bar label="RAM" :val="ru" :max="rt"
          :color="ru/rt > 0.8 ? '#ef4444' : ru/rt > 0.5 ? '#f59e0b' : '#22c55e'"
          :text="ru + '/' + rt + ' MB'"/>
      </div>

      <div class="card">
        <h3>Kênh truyền
          <div class="ch-badges" style="display:flex;gap:6px;flex-wrap:wrap">
            <span class="ch-lbl" style="font-size:.7rem;color:#64748b">MQTT:</span>
            <ch-badge :state="m.state" :enabled="m.enabled"/>
            <span class="ch-lbl" style="font-size:.7rem;color:#64748b;margin-left:6px">HTTP:</span>
            <ch-badge :state="hp.state" :enabled="hp.enabled"/>
            <span class="ch-lbl" style="font-size:.7rem;color:#64748b;margin-left:6px">TCP:</span>
            <ch-badge :state="t.state" :enabled="t.enabled"/>
          </div>
        </h3>
        <div class="cf">
          <span class="lbl">UART RX</span>
          <span style="color:#e2e8f0;font-weight:500">{{ u.rx_frames ?? 0 }} Frames / {{ u.rx_bytes ?? 0 }} Bytes</span>
          <span class="lbl">UART TX</span>
          <span style="color:#e2e8f0;font-weight:500">{{ u.tx_frames ?? 0 }} Frames / {{ u.tx_bytes ?? 0 }} Bytes</span>
          <span class="lbl">UART config</span>
          <span style="color:#e2e8f0;font-weight:500">{{ u.config || '-' }}</span>
          <span class="lbl">MQTT Pub</span>
          <span style="color:#e2e8f0;font-weight:500">{{ m.published ?? 0 }} ok / {{ m.failed ?? 0 }} fail</span>
          <span class="lbl">HTTP sent</span>
          <span style="color:#e2e8f0;font-weight:500">{{ hp.sent ?? 0 }} ok / {{ hp.failed ?? 0 }} fail</span>
          <span class="lbl">TCP conn</span>
          <span style="color:#e2e8f0;font-weight:500">{{ t.connections ?? 0 }}</span>
        </div>
      </div>

      <div v-if="ws && ws.mode !== 'off'" class="card">
        <h3>WiFi</h3>
        <div class="cf">
          <span class="lbl">Chế độ</span>
          <span>{{ ws.mode === 'sta' ? 'STA' : ws.mode === 'ap' ? 'AP' : 'STA + AP' }}</span>
          <template v-if="ws.mode !== 'ap'">
            <span class="lbl">STA</span>
            <span v-if="sta.connected" style="display:flex;align-items:center;gap:6px;color:#22c55e">
              <signal-bars :dbm="sta.signal"/> {{ sta.ssid || sta.config_ssid }} {{ sta.signal }} dBm
            </span>
            <span v-else style="color:#ef4444">
              {{ sta.config_ssid ? 'Ngắt — "' + sta.config_ssid + '"' : 'Chưa cấu hình' }}
            </span>
            <template v-if="sta.connected">
              <span class="lbl">IP</span><span>{{ sta.ip || '-' }}</span>
            </template>
          </template>
          <template v-if="ws.mode !== 'sta'">
            <span class="lbl">AP</span>
            <span :style="{ color: ap.active ? '#22c55e' : '#64748b' }">
              {{ ap.active ? ap.ssid : 'Tắt' }}
            </span>
          </template>
        </div>
      </div>

      <div class="card">
        <h3>GPIO</h3>
        <div class="gpio-btns">
          <div v-for="i in 4" :key="i"
               :class="'gpio-btn ' + (g[i-1] ? 'on' : 'off')"
               @click="sendGpio(i, 'toggle')">
            Pin {{ i }}<br>{{ g[i-1] ? 'ON' : 'OFF' }}
          </div>
        </div>
      </div>
    </div>
  `,
  setup() {
    const s = Vue.computed(() => store.status || {});
    const u = Vue.computed(() => s.value.uart || {});
    const m = Vue.computed(() => s.value.mqtt || {});
    const hp = Vue.computed(() => s.value.http || {});
    const t = Vue.computed(() => s.value.tcp || {});
    const g = Vue.computed(() => s.value.gpio || []);
    const cpu = Vue.computed(() => s.value.cpu || 0);
    const ru = Vue.computed(() => s.value.ram_used || 0);
    const rt = Vue.computed(() => s.value.ram_total || 1);
    const ws = Vue.computed(() => store.wifi.status);
    const sta = Vue.computed(() => (ws.value && ws.value.sta) || {});
    const ap = Vue.computed(() => (ws.value && ws.value.ap) || {});

    Vue.onMounted(() => {
      if (!store.wifi.status) loadWifiStatus();
    });

    return { store, s, u, m, hp, t, g, cpu, ru, rt, ws, sta, ap };
  },
  methods: {
    sendGpio(pin, state) {
      if (_ws && _ws.readyState === 1) {
        _ws.send(JSON.stringify({ cmd: 'gpio', pin: '' + pin, state }));
      }
    }
  }
};
