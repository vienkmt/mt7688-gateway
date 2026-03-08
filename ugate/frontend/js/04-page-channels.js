// --- Config page Vue component ---

const ConfigPage = {
  template: `
    <div v-if="!store.config" class="card">Đang tải...</div>
    <div v-else>
      <div class="card">
        <h3 style="display:flex;align-items:center;gap:8px">
          General <a href="#" class="help-link" @click.prevent="showDataWrapHelp()">Help</a>
        </h3>
        <div class="cf cf-3">
          <span class="lbl">Tên thiết bị</span>
          <input type="text" v-model="c.general.device_name">
          <span class="lbl">Data Wrap</span>
          <label class="chk">
            <input type="checkbox" v-model="c.general.wrap_json"
                   @change="c.general.wrap_json && showDataWrapHelp()">
            <span class="chk-box"></span>
            <span style="color:#e2e8f0;font-size:.85rem">{{ c.general.wrap_json ? 'Bật' : 'Tắt' }}</span>
          </label>
          <span class="lbl">Text encoding</span>
          <label class="chk">
            <input type="checkbox" v-model="c.general.data_as_text">
            <span class="chk-box"></span>
            <span style="color:#e2e8f0;font-size:.85rem">{{ c.general.data_as_text ? 'Text' : 'Hex' }}</span>
          </label>
        </div>
      </div>

      <div class="card">
        <channel-header title="MQTT" v-model="c.mqtt.enabled"/>
        <div v-if="c.mqtt.enabled" class="cf cf-3">
          <span class="lbl">Broker</span>
          <input type="text" v-model="c.mqtt.broker">
          <span class="lbl">Port</span>
          <input type="number" v-model.number="c.mqtt.port">
          <span class="lbl">TLS</span>
          <label class="chk">
            <input type="checkbox" v-model="c.mqtt.tls">
            <span class="chk-box"></span>
            <span style="color:#e2e8f0;font-size:.85rem">{{ c.mqtt.tls ? 'Bật' : 'Tắt' }}</span>
          </label>
          <span class="lbl">Client ID</span>
          <input type="text" readonly class="inp" style="opacity:.6;cursor:default"
                 :value="mqttClientId" placeholder="Chưa kết nối"
                 title="Tự sinh ngẫu nhiên mỗi lần kết nối">
          <span class="lbl">Username</span>
          <input type="text" v-model="c.mqtt.username">
          <span class="lbl">Mật khẩu</span>
          <input type="password" v-model="c.mqtt.password">
          <span class="lbl">QoS</span>
          <select v-model.number="c.mqtt.qos">
            <option :value="0">0</option><option :value="1">1</option><option :value="2">2</option>
          </select>
          <span class="lbl">Pub Topic</span>
          <input type="text" v-model="c.mqtt.topic">
          <span class="lbl">Sub Topic</span>
          <input type="text" v-model="c.mqtt.sub_topic">
        </div>
      </div>

      <div class="card">
        <channel-header title="HTTP" v-model="c.http.enabled"/>
        <div v-if="c.http.enabled" class="cf" style="grid-template-columns:auto 1fr auto 2fr">
          <span class="lbl">Phương thức</span>
          <select v-model="c.http.method">
            <option value="post">POST</option><option value="get">GET</option>
          </select>
          <span class="lbl">URL</span>
          <input type="text" v-model="c.http.url">
        </div>
      </div>

      <div class="card">
        <channel-header title="TCP" v-model="c.tcp.enabled"/>
        <div v-if="c.tcp.enabled" class="cf">
          <span class="lbl">Chế độ</span>
          <select v-model="c.tcp.mode">
            <option value="server">Server (lắng nghe)</option>
            <option value="client">Client (kết nối tới)</option>
          </select>
          <template v-if="c.tcp.mode === 'server'">
            <span class="lbl">Cổng lắng nghe</span>
            <input type="number" v-model.number="c.tcp.server_port">
          </template>
          <template v-if="c.tcp.mode === 'client'">
            <span class="lbl">Địa chỉ server</span>
            <input type="text" v-model="c.tcp.client_host">
            <span class="lbl">Cổng đích</span>
            <input type="number" v-model.number="c.tcp.client_port">
          </template>
        </div>
      </div>

      <button class="save-btn" @click="saveConfig">Lưu cấu hình</button>
    </div>
  `,
  setup() {
    const c = Vue.computed(() => store.config);
    const mqttClientId = Vue.computed(() => {
      const m = store.status && store.status.mqtt;
      return m ? m.client_id || '' : '';
    });
    Vue.onMounted(() => { if (!store.config) loadConfig(); });
    return { store, c, mqttClientId, showDataWrapHelp };
  },
  methods: {
    async saveConfig() {
      try {
        const r = await fetch('/api/config', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(store.config)
        });
        if (r.ok) toast('Đã lưu cấu hình', 'ok');
        else toast('Lưu thất bại', 'err');
      } catch (_) { toast('Lỗi kết nối', 'err'); }
    }
  }
};

async function loadConfig() {
  try {
    const r = await fetch('/api/config');
    if (r.ok) store.config = await r.json();
  } catch (_) {}
}
