// --- Vue 3 reactive store + global helpers ---

const store = Vue.reactive({
  page: 'login',
  connected: false,
  status: {},
  config: null,
  stream: [],
  loginErr: '',
  password: '',
  hexView: false,
  net: null,
  wifi: { networks: [], scanning: false, status: null },
  ntp: null,
  routes: null,
  wanIfaces: null,
  newRoute: {
    name: '',
    target: '',
    netmask: '255.255.255.0',
    gateway: '',
    interface: 'wan'
  },
  pendingChanges: false,
  sys: {
    version: null,
    updateInfo: null,
    checkingUpdate: false,
    upgradeUrl: ''
  },
  toolbox: null,
  syslog: null,
  _uartOpen: true,
  _diagOpen: true,
  _streamId: 0
});

let _ws = null;

// --- Toast ---
function toast(msg, type) {
  let t = document.getElementById('toast');
  if (!t) {
    t = document.createElement('div');
    t.id = 'toast';
    t.className = 'toast';
    document.body.append(t);
  }
  t.textContent = msg;
  t.className = 'toast ' + (type || 'ok') + ' show';
  clearTimeout(t._tid);
  t._tid = setTimeout(() => t.classList.remove('show'), 2500);
}

// --- Interface name mapping ---
const IFACE_MAP = {
  'eth0.2': 'ETH WAN',
  'eth0.2@eth0': 'ETH WAN',
  'phy0-sta0': 'WiFi WAN',
  'phy0-ap0': 'AP WiFi LAN',
  'br-lan': 'LAN Bridge',
  'lo': 'Loopback'
};

function ifaceName(raw) {
  return IFACE_MAP[raw] || raw;
}

function ifaceBadgeColor(raw) {
  const name = ifaceName(raw);
  return name.includes('ETH') ? '#3b82f6'
    : name.includes('WiFi WAN') ? '#f59e0b'
    : name.includes('AP') ? '#22c55e'
    : '#64748b';
}

function signalStrength(dbm) {
  return dbm > -50 ? 4 : dbm > -60 ? 3 : dbm > -70 ? 2 : 1;
}

function isSafeTarget(s) {
  return s.length > 0 && s.length <= 253 && !/^-/.test(s) && /^[a-zA-Z0-9.\-:]+$/.test(s);
}
