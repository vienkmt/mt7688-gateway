// --- Toolbox page Vue component ---

const ToolboxPage = {
  template: `
    <div style="display:flex;flex-direction:column;height:calc(100vh - 120px)">
      <div class="card" :style="'display:flex;flex-direction:column;' + (store._diagOpen ? 'flex:1;min-height:0;overflow:hidden' : '')">
        <h3>Network Diagnostics
          <button class="collapse-btn" @click="store._diagOpen = !store._diagOpen">
            <span class="arrow" :style="store._diagOpen ? 'transform:rotate(90deg)' : ''"
                  style="transition:transform .2s;display:inline-block">&#9654;</span>
          </button>
        </h3>
        <template v-if="store._diagOpen">
          <div class="diag-bar" style="display:flex;gap:8px;align-items:center;flex-wrap:wrap;margin-bottom:8px">
            <select v-model="tb.tool"
              style="padding:8px 12px;border:1px solid #334155;background:#0f172a;color:#e2e8f0;border-radius:6px;font-size:.85rem">
              <option v-for="t in ['ping','traceroute','nslookup']" :key="t" :value="t">{{ t }}</option>
            </select>
            <input type="text" v-model="tb.target" placeholder="hostname or IP"
              style="flex:1;padding:8px 12px;border:1px solid #334155;background:#0f172a;color:#e2e8f0;border-radius:6px;font-size:.85rem"
              @keydown.enter="!tb.running && tbRun()">
            <div class="diag-bar-btns" style="display:flex;gap:8px">
              <button class="save-btn" :style="'margin:0;padding:6px 16px;font-size:.8rem;' + (tb.running ? 'opacity:.5' : '')"
                      @click="!tb.running && tbRun()">{{ tb.running ? 'Running...' : 'Run' }}</button>
              <button v-if="tb.running"
                style="padding:6px 16px;background:#dc2626;color:white;border:none;border-radius:6px;cursor:pointer;font-size:.8rem;font-weight:700"
                @click="tbStop">Stop</button>
              <button style="padding:6px 16px;background:#334155;color:#94a3b8;border:1px solid #475569;border-radius:6px;cursor:pointer;font-size:.8rem"
                @click="tb.lines.splice(0)">Clear</button>
            </div>
          </div>
          <div ref="diagStream" class="stream" style="flex:1;overflow-y:auto;min-height:0">
            <div v-for="(l, i) in tb.lines" :key="i">{{ l }}</div>
          </div>
        </template>
      </div>

      <syslog-section/>
    </div>
  `,
  setup() {
    if (!store.toolbox) store.toolbox = { tool: 'ping', target: '', lines: [], running: false };
    const tb = store.toolbox;
    const diagStream = Vue.ref(null);

    Vue.watch(() => tb.lines.length, () => {
      Vue.nextTick(() => {
        if (diagStream.value) diagStream.value.scrollTop = diagStream.value.scrollHeight;
      });
    });

    return { store, tb, diagStream };
  },
  methods: {
    async tbRun() {
      const tb = this.tb;
      if (!tb.target.trim()) return;
      if (!isSafeTarget(tb.target.trim())) {
        toast('Target không hợp lệ (chỉ hostname/IP)', 'err'); return;
      }
      tb.lines.splice(0);
      tb.running = true;
      try {
        const r = await fetch('/api/toolbox/run', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ tool: tb.tool, target: tb.target })
        });
        if (!r.ok) {
          const d = await r.json();
          tb.lines.push('Error: ' + (d.error || 'failed'));
          tb.running = false;
        }
      } catch (e) {
        tb.lines.push('Error: ' + e.message);
        tb.running = false;
      }
    },
    async tbStop() {
      try { await fetch('/api/toolbox/stop', { method: 'POST' }); } catch (_) {}
    }
  }
};

const SyslogSection = {
  template: `
    <div class="card" style="display:flex;flex-direction:column;flex:2;min-height:0;overflow:hidden">
      <h3>System Log</h3>
      <div class="sl-bar" style="display:flex;gap:8px;align-items:center;flex-wrap:wrap;margin-bottom:8px">
        <input type="text" v-model="sl.filter" placeholder="Filter (ví dụ: MQTT, HTTP, error...)"
          style="flex:1;padding:8px 12px;border:1px solid #334155;background:#0f172a;color:#e2e8f0;border-radius:6px;font-size:.85rem">
        <div class="sl-bar-btns" style="display:flex;gap:8px">
          <button :style="'padding:6px 16px;background:' + (sl.running ? '#dc2626' : '#2563eb') + ';color:white;border:none;border-radius:6px;cursor:pointer;font-weight:700;font-size:.8rem'"
                  @click="sl.running ? slStop() : slStart()">{{ sl.running ? 'Stop' : 'Start' }}</button>
          <button style="padding:6px 16px;background:#334155;color:#94a3b8;border:1px solid #475569;border-radius:6px;cursor:pointer;font-size:.8rem"
                  @click="sl.lines.splice(0)">Clear</button>
        </div>
      </div>
      <div ref="slStream" class="stream" style="flex:1;overflow-y:auto;min-height:0">
        <div v-for="(l, i) in filtered" :key="i"
             :style="{ padding: '2px 0', borderBottom: '1px solid #1e293b',
                        color: l.level === 'err' ? '#f87171' : l.level === 'warn' ? '#facc15' : '' }">
          {{ l.text }}
        </div>
      </div>
    </div>
  `,
  setup() {
    if (!store.syslog) store.syslog = { lines: [], running: false, filter: '' };
    const sl = store.syslog;
    const slStream = Vue.ref(null);

    const filtered = Vue.computed(() => {
      if (!sl.filter) return sl.lines;
      const f = sl.filter.toLowerCase();
      return sl.lines.filter(l => l.text.toLowerCase().includes(f));
    });

    Vue.watch(() => sl.lines.length, () => {
      Vue.nextTick(() => {
        if (slStream.value) slStream.value.scrollTop = slStream.value.scrollHeight;
      });
    });

    return { store, sl, slStream, filtered };
  },
  methods: {
    async slStart() {
      this.sl.running = true;
      try {
        const r = await fetch('/api/syslog/start', { method: 'POST' });
        if (!r.ok) this.sl.running = false;
      } catch (_) { this.sl.running = false; }
    },
    async slStop() {
      try { await fetch('/api/syslog/stop', { method: 'POST' }); } catch (_) {}
      this.sl.running = false;
    }
  }
};
