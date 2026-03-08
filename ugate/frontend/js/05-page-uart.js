// --- UART page Vue component ---

const UartPage = {
  template: `
    <div style="display:flex;flex-direction:column;height:calc(100vh - 120px)">
      <div class="card" style="flex-shrink:0">
        <h3>Cài đặt UART
          <button class="collapse-btn" @click="store._uartOpen = !store._uartOpen">
            <span class="arrow" :style="store._uartOpen ? 'transform:rotate(90deg)' : ''"
                  style="transition:transform .2s;display:inline-block">&#9654;</span>
          </button>
        </h3>
        <div v-if="store._uartOpen && store.config">
          <div class="cf">
            <span class="lbl">Baudrate</span>
            <select v-model="cfg.baudrate">
              <option v-for="b in bauds" :key="b" :value="b">{{ b }}</option>
            </select>
            <span class="lbl">Data Bits</span>
            <select v-model="cfg.data_bits">
              <option value="7">7</option><option value="8">8</option>
            </select>
            <span class="lbl">Parity</span>
            <select v-model="cfg.parity">
              <option value="none">None</option><option value="even">Even</option><option value="odd">Odd</option>
            </select>
            <span class="lbl">Stop Bits</span>
            <select v-model="cfg.stop_bits">
              <option value="1">1</option><option value="2">2</option>
            </select>
            <span class="lbl">Frame Mode</span>
            <select v-model="cfg.frame_mode">
              <option value="none">None (Gap)</option>
              <option value="frame">Frame (Fixed)</option>
              <option value="modbus">Modbus RTU</option>
            </select>
            <template v-if="cfg.frame_mode === 'none'">
              <span class="lbl">Gap (ms)</span>
              <input type="number" v-model.number="cfg.gap_ms">
            </template>
            <template v-if="cfg.frame_mode === 'frame'">
              <span class="lbl">Frame Length</span>
              <input type="number" v-model.number="cfg.frame_length">
              <span class="lbl">Frame Timeout (ms)</span>
              <input type="number" v-model.number="cfg.frame_timeout_ms">
            </template>
            <template v-if="cfg.frame_mode === 'modbus'">
              <span class="lbl">Frame Timeout (ms)</span>
              <input type="number" v-model.number="cfg.frame_timeout_ms">
            </template>
          </div>
          <button class="save-btn" @click="saveUartConfig">Lưu cấu hình</button>
        </div>
        <div v-else-if="!store.config">Đang tải...</div>
      </div>

      <div class="card" style="flex:1;display:flex;flex-direction:column;padding-bottom:8px;min-height:0;overflow:hidden">
        <div style="display:flex;align-items:center;gap:40px;margin-bottom:8px">
          <h3 style="margin:0;font-size:.9rem;color:#94a3b8;text-transform:uppercase;letter-spacing:.05em;white-space:nowrap">
            UART Real-time
          </h3>
          <div style="display:flex;align-items:center;gap:6px;flex:1;min-width:0;background:#1e293b;border:1px solid #334155;border-radius:6px;padding:3px 4px">
            <input ref="txInput" type="text" placeholder="Gửi serial..."
                   style="flex:1;padding:2px 6px;background:transparent;color:#e2e8f0;border:none;outline:none;font-size:.8rem;font-family:monospace;min-width:0"
                   @keydown.enter="sendTx">
            <button style="padding:3px 12px;background:#2563eb;color:white;border:none;border-radius:4px;cursor:pointer;font-size:.78rem;font-weight:600;white-space:nowrap"
                    @click="sendTx">Gửi</button>
          </div>
          <label class="chk" style="font-size:.78rem">
            <input type="checkbox" v-model="store.hexView">
            <span class="chk-box"></span>
            <span style="color:#94a3b8">HEX</span>
          </label>
          <button style="padding:3px 10px;background:#334155;color:#94a3b8;border:1px solid #475569;border-radius:4px;cursor:pointer;font-size:.75rem"
                  @click="clearStream">Xoá</button>
        </div>
        <div ref="streamEl" class="stream" style="flex:1;overflow-y:auto;min-height:0">
          <div v-for="d in store.stream" :key="d._id"
               style="display:flex;gap:8px;padding:1px 0;border-bottom:1px solid #1e293b">
            <span style="color:#64748b;min-width:64px">{{ d._ts || '' }}</span>
            <span :style="{ color: d.dir === 'tx' ? '#f59e0b' : '#22c55e', fontWeight: 600, minWidth: '20px' }">
              {{ d.dir === 'tx' ? 'TX' : 'RX' }}
            </span>
            <span style="color:#38bdf8;min-width:28px">[{{ d.len }}]</span>
            <span>{{ formatContent(d) }}
              <span v-if="d.err" style="color:#ef4444;font-size:.75rem;margin-left:4px">({{ d.err }})</span>
            </span>
          </div>
        </div>
      </div>
    </div>
  `,
  setup() {
    const bauds = ['9600', '19200', '38400', '57600', '115200', '230400', '460800', '921600'];
    const cfg = Vue.computed(() => store.config ? store.config.uart : {});
    const streamEl = Vue.ref(null);

    Vue.watch(() => store.stream.length, () => {
      Vue.nextTick(() => {
        if (streamEl.value) streamEl.value.scrollTop = streamEl.value.scrollHeight;
      });
    });

    Vue.onMounted(() => { if (!store.config) loadConfig(); });

    return { store, bauds, cfg, streamEl };
  },
  methods: {
    formatContent(d) {
      if (!d.hex) return '';
      const bytes = d.hex.match(/.{1,2}/g) || [];
      if (store.hexView) return bytes.join(' ').toUpperCase();
      return bytes.map(b => {
        const c = parseInt(b, 16);
        return c >= 32 && c < 127 ? String.fromCharCode(c) : '';
      }).join('');
    },
    sendTx() {
      const input = this.$refs.txInput;
      const v = input.value.trim();
      if (!v) return;
      fetch('/api/uart/tx', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ data: v })
      });
      input.value = '';
    },
    clearStream() {
      store.stream.splice(0);
    },
    async saveUartConfig() {
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
