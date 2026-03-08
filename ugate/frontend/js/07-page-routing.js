// --- Routing page Vue component ---

const RoutingPage = {
  template: `
    <div>
      <pending-banner/>
      <metrics-section/>
      <routes-section/>
    </div>
  `,
  setup() {
    Vue.onMounted(() => { checkPendingChanges(); });
  }
};

const MetricsSection = {
  template: `
    <div v-if="!store.wanIfaces" class="card">Đang tải...</div>
    <div v-else-if="!store.wanIfaces.length" class="card">
      <h3>Ưu tiên mạng</h3>
      <div style="color:#64748b;font-size:.85rem">Không có WAN interface nào</div>
    </div>
    <div v-else class="card">
      <h3>Ưu tiên mạng
        <span style="font-size:.7rem;color:#64748b;font-weight:400;margin-left:8px">Metric thấp = ưu tiên cao</span>
      </h3>
      <div class="cf">
        <template v-for="ifc in store.wanIfaces" :key="ifc.uci">
          <span class="lbl">{{ ifc.label }}</span>
          <div style="display:flex;align-items:center;gap:8px">
            <input type="number" v-model.number="ifc._metric"
              style="width:80px;padding:6px 10px;border:1px solid #334155;background:#0f172a;color:#e2e8f0;border-radius:4px;font-size:.85rem">
            <span style="color:#64748b;font-size:.75rem">via {{ ifc.gateway }} ({{ ifc.dev }})</span>
          </div>
        </template>
      </div>
      <button class="save-btn" @click="saveMetrics">Lưu nháp</button>
    </div>
  `,
  setup() {
    Vue.onMounted(() => {
      if (!store.wanIfaces) loadWanIfaces();
    });
    return { store };
  },
  methods: {
    async saveMetrics() {
      if (!store.wanIfaces) return;
      for (const ifc of store.wanIfaces) {
        try {
          await fetch('/api/interface/metric', {
            method: 'POST',
            headers: { 'Content-Type': 'application/json' },
            body: JSON.stringify({ interface: ifc.uci, metric: +(ifc._metric || 0) })
          });
        } catch (_) {}
      }
      store.pendingChanges = true;
      toast('Đã lưu nháp (RAM). Ấn "Áp dụng" để ghi flash.', 'ok');
    }
  }
};

const RoutesSection = {
  template: `
    <div v-if="!store.routes" class="card">Đang tải...</div>
    <div v-else class="card">
      <h3>Routing</h3>
      <div style="font-weight:600;color:#38bdf8;margin-bottom:6px;font-size:.85rem">Bảng định tuyến</div>

      <div v-if="routes.length" class="route-wrap">
        <table style="width:100%;border-collapse:collapse;margin-bottom:12px">
          <thead>
            <tr>
              <th v-for="[t,w] in cols" :key="t"
                  :style="'text-align:left;padding:6px 8px;color:#64748b;font-size:.75rem;font-weight:600;border-bottom:1px solid #334155;width:'+w">
                {{ t }}
              </th>
            </tr>
          </thead>
          <tbody>
            <tr v-for="(rt, i) in routes" :key="i">
              <td style="padding:6px 8px;font-size:.8rem;color:#e2e8f0;font-family:monospace">{{ rt.dest }}</td>
              <td style="padding:6px 8px;font-size:.8rem;color:#94a3b8;font-family:monospace">{{ rt.via }}</td>
              <td style="padding:6px 8px"><iface-badge :dev="rt.dev"/></td>
              <td style="padding:6px 8px;font-size:.8rem;color:#94a3b8;text-align:center">{{ rt.metric }}</td>
              <td style="padding:6px 8px;font-size:.8rem;color:#64748b">{{ rt.scope }}</td>
            </tr>
          </tbody>
        </table>
      </div>
      <div v-else style="color:#64748b;font-size:.85rem;margin-bottom:12px">Chưa có route nào</div>

      <div style="font-weight:600;color:#38bdf8;margin-bottom:6px;font-size:.85rem">Thêm static route</div>
      <div class="cf">
        <span class="lbl">Tên</span>
        <input type="text" v-model="store.newRoute.name">
        <span class="lbl">Interface</span>
        <select v-model="store.newRoute.interface">
          <option value="wan">ETH WAN</option><option value="wwan">WiFi WAN</option>
        </select>
        <span class="lbl">Target</span>
        <input type="text" v-model="store.newRoute.target">
        <span class="lbl">Netmask</span>
        <input type="text" v-model="store.newRoute.netmask">
        <span class="lbl">Gateway</span>
        <input type="text" v-model="store.newRoute.gateway" class="full">
      </div>
      <button class="save-btn" @click="addRoute">Thêm route</button>
    </div>
  `,
  setup() {
    const routes = Vue.computed(() => (store.routes && store.routes.routes) || []);
    const cols = [['Đích','25%'],['Gateway','20%'],['Interface','20%'],['Metric','10%'],['Scope','10%']];
    Vue.onMounted(() => { if (!store.routes) loadRoutes(); });
    return { store, routes, cols };
  },
  methods: {
    async addRoute() {
      const r = store.newRoute;
      if (!r.name || !r.target || !r.gateway) {
        toast('Điền đầy đủ thông tin', 'err'); return;
      }
      try {
        const res = await fetch('/api/routes', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(r)
        });
        if (res.ok) {
          toast('Đã thêm route', 'ok');
          store.routes = null;
          store.newRoute = { name: '', target: '', netmask: '255.255.255.0', gateway: '', interface: 'wan' };
          loadRoutes();
        } else {
          const d = await res.json();
          toast(d.error || 'Lỗi', 'err');
        }
      } catch (_) { toast('Lỗi kết nối', 'err'); }
    }
  }
};

async function loadWanIfaces() {
  try {
    const r = await fetch('/api/wan/discover');
    if (r.ok) {
      const d = await r.json();
      const ifaces = d.interfaces || [];
      ifaces.forEach(ifc => {
        if (ifc._metric == null) ifc._metric = ifc.uci_metric || ifc.metric || '0';
      });
      store.wanIfaces = ifaces;
      return;
    }
  } catch (_) {}
  store.wanIfaces = [];
}

async function loadRoutes() {
  try {
    const r = await fetch('/api/routes');
    if (r.ok) { store.routes = await r.json(); return; }
  } catch (_) {}
  store.routes = { current_table: '' };
}
