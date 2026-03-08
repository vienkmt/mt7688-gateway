// --- Network page Vue component ---

const NetworkPage = {
  template: `
    <div>
      <pending-banner/>
      <wifi-section/>
      <lan-wan-section/>
      <ntp-section/>
    </div>
  `,
  setup() {
    Vue.onMounted(() => { checkPendingChanges(); });
  }
};

const WifiSection = {
  template: `
    <div class="card">
      <h3>WiFi
        <div style="display:flex;align-items:center;gap:8px">
          <select v-model="wMode"
            style="padding:4px 8px;border:1px solid #334155;background:#0f172a;color:#e2e8f0;border-radius:4px;font-size:.8rem">
            <option value="sta">STA (Client)</option>
            <option value="ap">AP (Phát WiFi)</option>
            <option value="sta_ap">STA + AP</option>
            <option value="off">Tắt WiFi</option>
          </select>
        </div>
      </h3>

      <div v-if="wMode === 'off'" style="color:#94a3b8;text-align:center;padding:16px;font-size:.85rem">
        WiFi đã tắt — chỉ dùng Ethernet
      </div>

      <div v-if="wMode === 'sta' || wMode === 'sta_ap'"
           style="background:#0f172a;border:1px solid #334155;border-radius:8px;padding:12px;margin-bottom:12px">
        <div style="display:flex;justify-content:space-between;align-items:center;margin-bottom:6px">
          <div style="font-weight:600;color:#38bdf8;font-size:.85rem;white-space:nowrap">STA — Kết nối WiFi</div>
          <button style="margin:0;padding:3px 8px;font-size:.9rem;background:#334155;color:#94a3b8;border:1px solid #475569;border-radius:4px;cursor:pointer;white-space:nowrap;flex-shrink:0"
                  @click="scanWifi">Quét WIFI</button>
        </div>
        <div v-if="sta.connected" class="cf" style="margin-bottom:8px">
          <span class="lbl">Trạng thái</span><span style="color:#22c55e">Đã kết nối</span>
          <span class="lbl">Tín hiệu</span>
          <span style="display:flex;align-items:center;gap:6px">
            <signal-bars :dbm="sta.signal"/> {{ sta.signal }} dBm
          </span>
          <span class="lbl">IP</span><span>{{ sta.ip || '-' }}</span>
        </div>
        <div v-else style="margin-bottom:8px;padding:6px 10px;background:#7f1d1d22;border:1px solid #7f1d1d;border-radius:6px;font-size:.8rem;color:#fca5a5">
          {{ sta.config_ssid ? 'Không kết nối được "' + sta.config_ssid + '" — kiểm tra SSID/mật khẩu hoặc router' : 'Chưa cấu hình WiFi' }}
        </div>
        <div class="cf">
          <span class="lbl">SSID</span>
          <input type="text" v-model="staSsid" placeholder="Nhập hoặc quét WiFi">
          <span class="lbl">Mật khẩu</span>
          <pwd-input v-model="staPwd" placeholder="Để trống nếu không cần"/>
        </div>
      </div>

      <div v-if="wMode === 'ap' || wMode === 'sta_ap'"
           style="background:#0f172a;border:1px solid #334155;border-radius:8px;padding:12px;margin-bottom:12px">
        <div style="font-weight:600;color:#22c55e;margin-bottom:6px;font-size:.85rem">
          AP — Phát WiFi{{ ap.active ? ' (đang phát)' : '' }}
        </div>
        <div class="cf">
          <span class="lbl">Tên WiFi</span>
          <input type="text" v-model="apSsid">
          <span class="lbl">Mật khẩu</span>
          <pwd-input v-model="apKey" placeholder="Để trống = WiFi mở"/>
          <template v-if="wMode === 'ap'">
            <span class="lbl">Kênh</span>
            <select v-model="apCh"
              style="padding:6px 10px;border:1px solid #334155;background:#0f172a;color:#e2e8f0;border-radius:4px;font-size:.85rem;width:100%">
              <option value="auto">Tự động</option>
              <option v-for="c in 11" :key="c" :value="String(c)">Kênh {{ c }}</option>
            </select>
          </template>
          <div v-else style="grid-column:1/-1;color:#64748b;font-size:.75rem;font-style:italic">
            Kênh AP tự theo STA khi ở chế độ STA+AP
          </div>
        </div>
      </div>

      <div style="display:flex;justify-content:center;margin-top:8px">
        <button class="save-btn" @click="saveDraft">Lưu nháp</button>
      </div>
    </div>
  `,
  setup() {
    const w = Vue.computed(() => store.wifi);
    const ws = Vue.computed(() => w.value.status || {});
    const sta = Vue.computed(() => ws.value.sta || {});
    const ap = Vue.computed(() => ws.value.ap || {});

    const wMode = Vue.ref('sta_ap');
    const staSsid = Vue.ref('');
    const staPwd = Vue.ref('');
    const apSsid = Vue.ref('');
    const apKey = Vue.ref('');
    const apCh = Vue.ref('auto');

    function syncFromStatus() {
      const s = ws.value;
      wMode.value = s.mode || 'sta_ap';
      staSsid.value = (s.sta && s.sta.config_ssid) || '';
      staPwd.value = (s.sta && s.sta.config_key) || '';
      apSsid.value = (s.ap && s.ap.ssid) || '';
      apKey.value = (s.ap && s.ap.key) || '';
      apCh.value = (s.ap && s.ap.channel) || 'auto';
    }

    Vue.watch(ws, (v) => { if (v && v.mode) syncFromStatus(); }, { immediate: true });

    Vue.onMounted(() => {
      if (!store.wifi.status) loadWifiStatus();
    });

    return { store, w, ws, sta, ap, wMode, staSsid, staPwd, apSsid, apKey, apCh };
  },
  methods: {
    async scanWifi() {
      const content = showWifiModal();
      content.innerHTML = '<div class="spinner"></div><div style="text-align:center;color:#94a3b8;margin-top:8px">Đang quét...</div>';
      try {
        const r = await fetch('/api/wifi/scan');
        if (r.ok) {
          const d = await r.json();
          store.wifi.networks = d.networks || [];
          this.renderWifiModal(content, store.wifi.networks);
        } else {
          content.innerHTML = '<div style="color:#ef4444;text-align:center;padding:20px">Lỗi quét WiFi</div>';
        }
      } catch (_) {
        content.innerHTML = '<div style="color:#ef4444;text-align:center;padding:20px">Lỗi kết nối</div>';
      }
    },
    renderWifiModal(content, networks) {
      content.innerHTML = '';
      if (!networks.length) {
        content.innerHTML = '<div style="color:#94a3b8;text-align:center;padding:20px">Không tìm thấy WiFi nào</div>';
        return;
      }
      networks.sort((a, b) => b.signal - a.signal);
      networks.forEach(n => {
        const item = document.createElement('div');
        item.className = 'wifi-item';
        // Signal bars SVG
        const str = signalStrength(n.signal);
        const bars = document.createElement('span');
        bars.className = 'wifi-signal';
        [6,10,14,18].forEach((ht, i) => {
          const bar = document.createElement('i');
          bar.style.height = ht + 'px';
          if (i < str) bar.className = 'active';
          bars.append(bar);
        });
        const left = document.createElement('div');
        const ssidEl = document.createElement('div');
        ssidEl.style = 'color:#e2e8f0;font-weight:500';
        ssidEl.textContent = n.ssid;
        const encEl = document.createElement('div');
        encEl.style = 'color:#64748b;font-size:.75rem';
        encEl.textContent = n.encryption || '';
        left.append(ssidEl, encEl);
        const right = document.createElement('div');
        right.style = 'display:flex;align-items:center;gap:8px';
        const dbmSpan = document.createElement('span');
        dbmSpan.style = 'color:#64748b;font-size:.75rem';
        dbmSpan.textContent = n.signal + ' dBm';
        right.append(bars, dbmSpan);
        item.append(left, right);
        item.onclick = () => {
          this.staSsid = n.ssid;
          this.staPwd = '';
          document.getElementById('wifi-modal').remove();
        };
        content.append(item);
      });
    },
    async saveDraft() {
      const body = { mode: this.wMode };
      if (this.wMode === 'sta' || this.wMode === 'sta_ap') {
        if (this.staSsid) body.sta_ssid = this.staSsid;
        body.sta_password = this.staPwd || '';
      }
      if (this.wMode === 'ap' || this.wMode === 'sta_ap') {
        if (this.apSsid) body.ap_ssid = this.apSsid;
        body.ap_password = this.apKey || '';
        if (this.apCh) body.ap_channel = this.apCh;
      }
      try {
        const r = await fetch('/api/wifi/mode', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(body)
        });
        if (r.ok) {
          toast('Đã lưu chế độ WiFi (draft)', 'ok');
          store.pendingChanges = true;
          setTimeout(() => loadWifiStatus(), 2000);
        } else toast('Lỗi lưu chế độ WiFi', 'err');
      } catch (_) { toast('Lỗi kết nối', 'err'); }
    }
  }
};

const PwdInput = {
  props: ['modelValue', 'placeholder'],
  emits: ['update:modelValue'],
  template: `
    <div style="position:relative;width:100%">
      <input :type="show ? 'text' : 'password'" :value="modelValue"
             :placeholder="placeholder || ''"
             style="width:100%;padding:6px 32px 6px 10px;border:1px solid #334155;background:#0f172a;color:#e2e8f0;border-radius:4px;font-size:.85rem"
             @input="$emit('update:modelValue', $event.target.value)">
      <span style="position:absolute;right:8px;top:50%;transform:translateY(-50%);cursor:pointer;color:#64748b;font-size:.8rem;user-select:none"
            @click="show = !show">{{ show ? '◡' : '◉' }}</span>
    </div>
  `,
  data() { return { show: false }; }
};

const LanWanSection = {
  template: `
    <div v-if="!store.net" class="card">Đang tải...</div>
    <div v-else class="card">
      <h3>ETH WAN
        <span style="font-size:.7rem;color:#64748b;font-weight:400;margin-left:8px">eth0.2</span>
      </h3>
      <div class="cf">
        <span class="lbl">Giao thức</span>
        <select v-model="store.net.wan.proto" class="full">
          <option value="dhcp">DHCP</option><option value="static">Static IP</option>
        </select>
        <template v-if="store.net.wan.proto === 'static'">
          <span class="lbl">IP</span>
          <input type="text" v-model="store.net.wan.ipaddr" class="full">
          <span class="lbl">Netmask</span>
          <input type="text" v-model="store.net.wan.netmask">
          <span class="lbl">Gateway</span>
          <input type="text" v-model="store.net.wan.gateway">
          <span class="lbl">DNS</span>
          <input type="text" v-model="wanDns" class="full">
        </template>
      </div>
      <button class="save-btn" @click="saveNetwork">Lưu nháp</button>
    </div>
  `,
  setup() {
    const wanDns = Vue.ref('');
    Vue.watch(() => store.net, (n) => {
      if (n && n.wan) wanDns.value = (n.wan.dns || []).join(',');
    }, { immediate: true });

    Vue.onMounted(() => { if (!store.net) loadNetwork(); });

    return { store, wanDns };
  },
  methods: {
    async saveNetwork() {
      const w = store.net.wan;
      const body = { interface: 'wan', proto: w.proto };
      if (w.proto === 'static') {
        body.ipaddr = w.ipaddr;
        body.netmask = w.netmask;
        body.gateway = w.gateway;
        body.dns = this.wanDns || '';
      }
      try {
        const r = await fetch('/api/network', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(body)
        });
        if (!r.ok) { toast('Lỗi lưu WAN', 'err'); return; }
      } catch (_) { toast('Lỗi kết nối', 'err'); return; }
      store.pendingChanges = true;
      toast('Đã lưu nháp (RAM)', 'ok');
    }
  }
};

const NtpSection = {
  template: `
    <div v-if="!store.ntp" class="card">Đang tải...</div>
    <div v-else class="card">
      <h3 style="display:flex;align-items:center;gap:10px">NTP
        <switch-toggle v-model="store.ntp.enabled"/>
      </h3>
      <div class="cf">
        <span class="lbl">Timezone</span>
        <select v-model="store.ntp.timezone">
          <option v-if="store.ntp.timezone && !tzMap[store.ntp.timezone]"
                  :value="store.ntp.timezone">{{ store.ntp.timezone }} (custom)</option>
          <option v-for="[v,t] in tzOpts" :key="v" :value="v">{{ t }}</option>
        </select>
        <template v-for="(sv, i) in srvs" :key="i">
          <span class="lbl">Server {{ i+1 }}</span>
          <input type="text" v-model="srvs[i]" placeholder="pool.ntp.org">
        </template>
        <div style="grid-column:1/-1;display:flex;gap:8px">
          <button style="padding:3px 10px;background:#1e3a5f;color:#93c5fd;border:1px solid #334155;border-radius:4px;cursor:pointer;font-size:.75rem"
                  @click="srvs.push('')">+ Thêm</button>
          <button v-if="srvs.length > 1"
                  style="padding:3px 10px;background:#991b1b;color:#fca5a5;border:none;border-radius:4px;cursor:pointer;font-size:.75rem"
                  @click="srvs.pop()">- Xoá cuối</button>
        </div>
      </div>
      <div style="display:flex;gap:8px;justify-content:center;margin-top:8px">
        <button class="save-btn" style="margin:0;background:#334155" @click="syncNtp">Đồng bộ ngay</button>
        <button class="save-btn" style="margin:0" @click="saveNtp">Lưu NTP</button>
      </div>
    </div>
  `,
  setup() {
    const srvs = Vue.reactive([]);
    const tzOpts = [
      // UTC/GMT
      ['UTC0', 'UTC'],
      ['GMT0', 'GMT'],
      // Asia
      ['<+07>-7', 'Asia/Ho_Chi_Minh (UTC+7)'],
      ['ICT-7', 'Asia/Bangkok (UTC+7)'],
      ['WIB-7', 'Asia/Jakarta (UTC+7)'],
      ['CST-8', 'Asia/Shanghai (UTC+8)'],
      ['<+08>-8', 'Asia/Singapore (UTC+8)'],
      ['PHT-8', 'Asia/Manila (UTC+8)'],
      ['<+08>-8', 'Asia/Kuala_Lumpur (UTC+8)'],
      ['HKT-8', 'Asia/Hong_Kong (UTC+8)'],
      ['CST-8', 'Asia/Taipei (UTC+8)'],
      ['JST-9', 'Asia/Tokyo (UTC+9)'],
      ['KST-9', 'Asia/Seoul (UTC+9)'],
      ['<+09>-9', 'Asia/Pyongyang (UTC+9)'],
      ['<+05>-5', 'Asia/Tashkent (UTC+5)'],
      ['<+05:30>-5:30', 'Asia/Kolkata (UTC+5:30)'],
      ['<+05:45>-5:45', 'Asia/Kathmandu (UTC+5:45)'],
      ['<+06>-6', 'Asia/Dhaka (UTC+6)'],
      ['<+06:30>-6:30', 'Asia/Yangon (UTC+6:30)'],
      ['<+04>-4', 'Asia/Dubai (UTC+4)'],
      ['<+03>-3', 'Asia/Baghdad (UTC+3)'],
      ['<+03:30>-3:30', 'Asia/Tehran (UTC+3:30)'],
      ['IST-2IDT,M3.4.4/26,M10.5.0', 'Asia/Jerusalem (UTC+2)'],
      ['<+03>-3', 'Asia/Riyadh (UTC+3)'],
      // Europe
      ['WET0WEST,M3.5.0/1,M10.5.0', 'Europe/London (UTC+0/+1)'],
      ['WET0WEST,M3.5.0/1,M10.5.0', 'Europe/Lisbon (UTC+0/+1)'],
      ['CET-1CEST,M3.5.0,M10.5.0/3', 'Europe/Paris (UTC+1/+2)'],
      ['CET-1CEST,M3.5.0,M10.5.0/3', 'Europe/Berlin (UTC+1/+2)'],
      ['CET-1CEST,M3.5.0,M10.5.0/3', 'Europe/Rome (UTC+1/+2)'],
      ['CET-1CEST,M3.5.0,M10.5.0/3', 'Europe/Madrid (UTC+1/+2)'],
      ['CET-1CEST,M3.5.0,M10.5.0/3', 'Europe/Amsterdam (UTC+1/+2)'],
      ['CET-1CEST,M3.5.0,M10.5.0/3', 'Europe/Warsaw (UTC+1/+2)'],
      ['EET-2EEST,M3.5.0/3,M10.5.0/4', 'Europe/Athens (UTC+2/+3)'],
      ['EET-2EEST,M3.5.0/3,M10.5.0/4', 'Europe/Bucharest (UTC+2/+3)'],
      ['EET-2EEST,M3.5.0/3,M10.5.0/4', 'Europe/Helsinki (UTC+2/+3)'],
      ['EET-2EEST,M3.5.0/3,M10.5.0/4', 'Europe/Istanbul (UTC+2/+3)'],
      ['<+03>-3', 'Europe/Moscow (UTC+3)'],
      // Americas
      ['AST4ADT,M3.2.0,M11.1.0', 'America/Halifax (Atlantic)'],
      ['EST5EDT,M3.2.0,M11.1.0', 'America/New_York (Eastern)'],
      ['EST5EDT,M3.2.0,M11.1.0', 'America/Toronto (Eastern)'],
      ['CST6CDT,M3.2.0,M11.1.0', 'America/Chicago (Central)'],
      ['CST6CDT,M3.2.0,M11.1.0', 'America/Mexico_City (Central)'],
      ['MST7MDT,M3.2.0,M11.1.0', 'America/Denver (Mountain)'],
      ['MST7', 'America/Phoenix (Arizona, no DST)'],
      ['PST8PDT,M3.2.0,M11.1.0', 'America/Los_Angeles (Pacific)'],
      ['PST8PDT,M3.2.0,M11.1.0', 'America/Vancouver (Pacific)'],
      ['AKST9AKDT,M3.2.0,M11.1.0', 'America/Anchorage (Alaska)'],
      ['<-03>3', 'America/Sao_Paulo (UTC-3)'],
      ['<-03>3', 'America/Buenos_Aires (UTC-3)'],
      ['<-05>5', 'America/Lima (UTC-5)'],
      ['<-05>5', 'America/Bogota (UTC-5)'],
      // Australia / Pacific
      ['AWST-8', 'Australia/Perth (UTC+8)'],
      ['ACST-9:30ACDT,M10.1.0,M4.1.0/3', 'Australia/Adelaide (UTC+9:30)'],
      ['AEST-10', 'Australia/Brisbane (UTC+10, no DST)'],
      ['AEST-10AEDT,M10.1.0,M4.1.0/3', 'Australia/Sydney (UTC+10/+11)'],
      ['NZST-12NZDT,M9.5.0,M4.1.0/3', 'Pacific/Auckland (UTC+12/+13)'],
      ['HST10', 'Pacific/Honolulu (Hawaii, UTC-10)'],
      ['<+12>-12', 'Pacific/Fiji (UTC+12)'],
    ];

    Vue.watch(() => store.ntp, (n) => {
      if (n) {
        srvs.splice(0, srvs.length, ...(n.servers && n.servers.length ? n.servers : ['']));
      }
    }, { immediate: true });

    Vue.onMounted(() => { if (!store.ntp) loadNtp(); });

    const tzMap = Object.fromEntries(tzOpts);
    return { store, srvs, tzOpts, tzMap };
  },
  methods: {
    async saveNtp() {
      const filtered = this.srvs.filter(s => s.trim());
      const body = {
        enabled: !!store.ntp.enabled,
        timezone: store.ntp.timezone || '',
        zonename: store.ntp.zonename || '',
        servers: filtered.join(',')
      };
      try {
        const r = await fetch('/api/ntp', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify(body)
        });
        if (r.ok) toast('Đã lưu NTP', 'ok');
        else toast('Lưu NTP lỗi', 'err');
      } catch (_) { toast('Lỗi kết nối', 'err'); }
    },
    async syncNtp() {
      try {
        const r = await fetch('/api/ntp/sync', { method: 'POST' });
        if (r.ok) {
          const d = await r.json();
          toast('Đã đồng bộ (' + d.method + ')', 'ok');
        } else toast('Lỗi sync', 'err');
      } catch (_) { toast('Lỗi kết nối', 'err'); }
    }
  }
};

// --- Network API helpers ---

function showWifiModal() {
  const existing = document.getElementById('wifi-modal');
  if (existing) existing.remove();
  const overlay = document.createElement('div');
  overlay.className = 'modal-overlay';
  overlay.id = 'wifi-modal';
  overlay.onclick = e => { if (e.target === overlay) overlay.remove(); };
  const modal = document.createElement('div');
  modal.className = 'modal';
  modal.style.position = 'relative';
  const content = document.createElement('div');
  content.id = 'wifi-modal-content';
  const h3 = document.createElement('h3');
  h3.textContent = 'Quét WiFi';
  const closeBtn = document.createElement('button');
  closeBtn.className = 'modal-close';
  closeBtn.textContent = '\u2715';
  closeBtn.onclick = () => overlay.remove();
  modal.append(h3, closeBtn, content);
  overlay.append(modal);
  document.body.append(overlay);
  return content;
}

async function loadWifiStatus() {
  try {
    const r = await fetch('/api/wifi/status');
    if (r.ok) { store.wifi.status = await r.json(); return; }
  } catch (_) {}
  if (!store.wifi.status) {
    store.wifi.status = {
      mode: 'sta_ap',
      sta: { connected: false, ssid: '', config_ssid: '', signal: 0, ip: '' },
      ap: { active: false, ssid: '', encryption: '', key: '', channel: '' }
    };
  }
}

async function loadNetwork() {
  try {
    const r = await fetch('/api/network');
    if (r.ok) { store.net = await r.json(); return; }
  } catch (_) {}
  store.net = {
    lan: { name: 'lan', proto: 'dhcp', ipaddr: '', netmask: '255.255.255.0', gateway: '', dns: [] },
    wan: { name: 'wan', proto: 'dhcp', ipaddr: '', netmask: '255.255.255.0', gateway: '', dns: [] }
  };
}

async function loadNtp() {
  try {
    const r = await fetch('/api/ntp');
    if (r.ok) { store.ntp = await r.json(); return; }
  } catch (_) {}
  store.ntp = { enabled: true, servers: [], timezone: 'ICT-7', zonename: 'Asia/Ho_Chi_Minh' };
}

async function checkPendingChanges() {
  try {
    const r = await fetch('/api/network/changes');
    if (r.ok) {
      const d = await r.json();
      store.pendingChanges = !!d.pending;
    }
  } catch (_) {}
}
