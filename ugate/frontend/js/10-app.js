// --- Vue app mount + WebSocket ---

const LoginPage = {
  template: `
    <div class="login">
      <h1 style="color:#38bdf8;font-size:1.5rem">uGate - From UART to the world</h1>
      <input ref="pwInput" type="password" placeholder="Mật khẩu"
             v-model="store.password" @keydown.enter="doLogin">
      <button @click="doLogin">Đăng nhập</button>
      <div v-if="store.loginErr" class="error">{{ store.loginErr }}</div>
    </div>
  `,
  setup() {
    Vue.onMounted(() => {
      Vue.nextTick(() => {
        const el = document.querySelector('.login input');
        if (el) el.focus();
      });
    });
    return { store };
  },
  methods: {
    async doLogin() {
      try {
        const r = await fetch('/api/login', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ password: store.password })
        });
        if (r.ok) {
          store.page = 'status';
          store.loginErr = '';
          connectWS();
        } else {
          store.loginErr = 'Sai mật khẩu';
        }
      } catch (_) { store.loginErr = 'Lỗi kết nối'; }
    }
  }
};

// --- Create & register Vue app ---
const app = Vue.createApp({
  template: `
    <template v-if="store.page === 'login'">
      <login-page/>
    </template>
    <template v-else>
      <app-header/>
      <nav-bar/>
      <component :is="store.page + '-page'"/>
    </template>
  `,
  setup() { return { store }; }
});

// Shared components
app.component('signal-bars', SignalBars);
app.component('iface-badge', IfaceBadge);
app.component('ch-badge', ChBadge);
app.component('progress-bar', ProgressBar);
app.component('switch-toggle', SwitchToggle);
app.component('channel-header', ChannelHeader);
app.component('pending-banner', PendingBanner);
app.component('pwd-input', PwdInput);

// Layout
app.component('app-header', AppHeader);
app.component('nav-bar', NavBar);
app.component('login-page', LoginPage);

// Pages
app.component('status-page', StatusPage);
app.component('config-page', ConfigPage);
app.component('uart-page', UartPage);
app.component('network-page', NetworkPage);
app.component('wifi-section', WifiSection);
app.component('lan-wan-section', LanWanSection);
app.component('ntp-section', NtpSection);
app.component('routing-page', RoutingPage);
app.component('metrics-section', MetricsSection);
app.component('routes-section', RoutesSection);
app.component('toolbox-page', ToolboxPage);
app.component('syslog-section', SyslogSection);
app.component('system-page', SystemPage);

app.mount('#app');

// --- WebSocket ---
function connectWS() {
  if (_ws) _ws.close();
  const ws = new WebSocket('ws://' + location.host + '/ws');

  ws.onopen = () => { store.connected = true; };
  ws.onclose = () => {
    store.connected = false;
    setTimeout(connectWS, 3000);
  };

  ws.onmessage = e => {
    try {
      const d = JSON.parse(e.data);
      if (d.type === 'status') {
        store.status = d;
      } else if (d.type === 'toolbox') {
        if (!store.toolbox) store.toolbox = { tool: 'ping', target: '', lines: [], running: false };
        if (d.done) {
          store.toolbox.running = false;
          store.toolbox.lines.push('--- done (exit code: ' + d.code + ') ---');
        } else if (d.line != null) {
          store.toolbox.lines.push(d.line);
        }
      } else if (d.type === 'syslog') {
        if (!store.syslog) store.syslog = { lines: [], running: false, filter: '' };
        if (d.stopped) {
          store.syslog.running = false;
        } else if (d.line != null) {
          store.syslog.lines.push({ text: d.line, level: d.level || 'info' });
          if (store.syslog.lines.length > 200) store.syslog.lines.shift();
        }
      } else {
        // UART data
        d._ts = new Date().toLocaleTimeString('vi', {
          hour12: false, hour: '2-digit', minute: '2-digit', second: '2-digit'
        });
        d._id = ++store._streamId;
        store.stream.push(d);
        if (store.stream.length > 200) store.stream.shift();
      }
    } catch (_) {}
  };

  _ws = ws;
}

// --- Auto-check session on load ---
(async function checkSession() {
  try {
    const r = await fetch('/api/status');
    if (r.ok) {
      store.page = 'status';
      connectWS();
    }
  } catch (_) {}
})();
