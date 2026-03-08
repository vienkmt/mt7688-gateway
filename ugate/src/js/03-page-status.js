// --- Status page ---

function renderWifiStatus() {
  if (!S.wifi.status) {
    loadWifiStatus().then(render);
    return h('div', {});
  }
  const ws = S.wifi.status;
  if (ws.mode === 'off') return h('div', {});

  const sta = ws.sta || {};
  const ap = ws.ap || {};
  const items = [];

  items.push(
    lbl('Chế độ'),
    h('span', {}, ws.mode === 'sta' ? 'STA' : ws.mode === 'ap' ? 'AP' : 'STA + AP')
  );

  if (ws.mode !== 'ap') {
    const staVal = sta.connected
      ? h('span', { style: 'display:flex;align-items:center;gap:6px;color:#22c55e' },
          signalBars(sta.signal), sta.ssid || sta.config_ssid, ' ', sta.signal + ' dBm')
      : h('span', { style: 'color:#ef4444' },
          sta.config_ssid ? 'Ngắt — "' + sta.config_ssid + '"' : 'Chưa cấu hình');
    items.push(lbl('STA'), staVal);
    if (sta.connected) items.push(lbl('IP'), h('span', {}, sta.ip || '-'));
  }

  if (ws.mode !== 'sta') {
    items.push(
      lbl('AP'),
      h('span', { style: 'color:' + (ap.active ? '#22c55e' : '#64748b') },
        ap.active ? ap.ssid : 'Tắt')
    );
  }

  return h('div', { cls: 'card' },
    h('h3', {}, 'WiFi'),
    h('div', { cls: 'cf' }, ...items)
  );
}

function renderStatus() {
  const s = S.status || {};
  const u = s.uart || {};
  const m = s.mqtt || {};
  const hp = s.http || {};
  const t = s.tcp || {};
  const g = s.gpio || [];
  const cpu = s.cpu || 0;
  const ru = s.ram_used || 0;
  const rt = s.ram_total || 0;

  function badge(ch, state, enabled) {
    const cls = enabled === false || state === 'disabled' ? 'disabled'
      : state === 'connected' ? 'connected' : 'disconnected';
    const txt = enabled === false ? 'off' : state || 'disconnected';
    return h('span', { cls: 'ch-badge ' + cls, 'data-badge': ch }, txt);
  }

  return h('div', {},
    h('div', { cls: 'card' },
      h('h3', {}, 'Hệ thống'),
      h('div', {
        cls: 'sys-info cf',
        style: 'grid-template-columns:auto 1fr auto 1fr auto 1fr'
      },
        lbl('Phiên bản'),
        h('span', { style: 'color:#e2e8f0;font-weight:500', 'data-f': 'ver' }, s.version || '-'),
        lbl('Uptime'),
        h('span', { style: 'color:#e2e8f0;font-weight:500', 'data-f': 'up' }, s.uptime || '-'),
        lbl('Thời gian'),
        h('span', { style: 'color:#e2e8f0;font-weight:500', 'data-f': 'dt' }, s.datetime || '-')
      ),
      pbar('cpu', 'CPU', cpu, 100,
        cpu > 80 ? '#ef4444' : cpu > 50 ? '#f59e0b' : '#22c55e', cpu + '%'),
      pbar('ram', 'RAM', ru, rt,
        ru / rt > 0.8 ? '#ef4444' : ru / rt > 0.5 ? '#f59e0b' : '#22c55e',
        ru + '/' + rt + ' MB')
    ),
    h('div', { cls: 'card' },
      h('h3', {}, 'Kênh truyền',
        h('div', {
          cls: 'ch-badges',
          style: 'display:flex;gap:6px;flex-wrap:wrap'
        },
          h('span', { cls: 'ch-lbl', style: 'font-size:.7rem;color:#64748b' }, 'MQTT:'),
          badge('mqtt', m.state, m.enabled),
          h('span', { cls: 'ch-lbl', style: 'font-size:.7rem;color:#64748b;margin-left:6px' }, 'HTTP:'),
          badge('http', hp.state, hp.enabled),
          h('span', { cls: 'ch-lbl', style: 'font-size:.7rem;color:#64748b;margin-left:6px' }, 'TCP:'),
          badge('tcp', t.state, t.enabled)
        )
      ),
      h('div', { cls: 'cf' },
        lbl('UART RX'),
        h('span', { style: 'color:#e2e8f0;font-weight:500', 'data-f': 'urx' },
          '' + (u.rx_frames ?? 0) + ' fr / ' + (u.rx_bytes ?? 0) + ' B'),
        lbl('UART TX'),
        h('span', { style: 'color:#e2e8f0;font-weight:500', 'data-f': 'utx' },
          '' + (u.tx_frames ?? 0) + ' fr / ' + (u.tx_bytes ?? 0) + ' B'),
        lbl('UART config'),
        h('span', { style: 'color:#e2e8f0;font-weight:500', 'data-f': 'ucfg' }, u.config || '-'),
        lbl('MQTT pub'),
        h('span', { style: 'color:#e2e8f0;font-weight:500', 'data-f': 'mpub' },
          '' + (m.published ?? 0) + ' ok / ' + (m.failed ?? 0) + ' fail'),
        lbl('HTTP sent'),
        h('span', { style: 'color:#e2e8f0;font-weight:500', 'data-f': 'hsnt' },
          '' + (hp.sent ?? 0) + ' ok / ' + (hp.failed ?? 0) + ' fail'),
        lbl('TCP conn'),
        h('span', { style: 'color:#e2e8f0;font-weight:500', 'data-f': 'tconn' },
          '' + (t.connections ?? 0))
      )
    ),
    renderWifiStatus(),
    h('div', { cls: 'card' },
      h('h3', {}, 'GPIO'),
      h('div', { cls: 'gpio-btns' },
        ...[0, 1, 2, 3].map(i =>
          h('div', {
            cls: 'gpio-btn ' + (g[i] ? 'on' : 'off'),
            'data-gpio': '' + i,
            onclick: () => sendGpio(i + 1, 'toggle')
          }, 'Pin ' + (i + 1) + '\n' + (g[i] ? 'ON' : 'OFF'))
        )
      )
    )
  );
}

function updateStatus() {
  // First render: full render; subsequent: in-place update for smooth transitions
  if (!document.querySelector('[data-pbar="cpu"]')) {
    render();
    return;
  }

  const s = S.status || {};
  const u = s.uart || {};
  const m = s.mqtt || {};
  const hp = s.http || {};
  const t = s.tcp || {};
  const g = s.gpio || [];
  const cpu = s.cpu || 0;
  const ru = s.ram_used || 0;
  const rt = s.ram_total || 0;

  // Update progress bars (reuse existing elements for animation)
  pbar('cpu', 'CPU', cpu, 100,
    cpu > 80 ? '#ef4444' : cpu > 50 ? '#f59e0b' : '#22c55e', cpu + '%');
  pbar('ram', 'RAM', ru, rt,
    ru / rt > 0.8 ? '#ef4444' : ru / rt > 0.5 ? '#f59e0b' : '#22c55e',
    ru + '/' + rt + ' MB');

  // Update text fields via data-field
  const f = k => document.querySelector('[data-f="' + k + '"]');
  const set = (k, v) => { const el = f(k); if (el) el.textContent = v };

  set('ver', s.version || '-');
  set('up', s.uptime || '-');
  set('dt', s.datetime || '-');
  set('urx', '' + (u.rx_frames ?? 0) + ' frames / ' + (u.rx_bytes ?? 0) + ' B');
  set('utx', '' + (u.tx_frames ?? 0) + ' frames / ' + (u.tx_bytes ?? 0) + ' B');
  set('ucfg', u.config || '-');
  set('mpub', '' + (m.published ?? 0) + ' ok / ' + (m.failed ?? 0) + ' fail');
  set('hsnt', '' + (hp.sent ?? 0) + ' ok / ' + (hp.failed ?? 0) + ' fail');
  set('tconn', '' + (t.connections ?? 0));

  // Update GPIO buttons
  [0, 1, 2, 3].forEach(i => {
    const btn = document.querySelector('[data-gpio="' + i + '"]');
    if (btn) {
      btn.className = 'gpio-btn ' + (g[i] ? 'on' : 'off');
      btn.textContent = 'Pin ' + (i + 1) + '\n' + (g[i] ? 'ON' : 'OFF');
    }
  });

  // Update badges
  ['mqtt', 'http', 'tcp'].forEach(ch => {
    const el = document.querySelector('[data-badge="' + ch + '"]');
    if (!el) return;
    const obj = ch === 'mqtt' ? m : ch === 'http' ? hp : t;
    const st = obj.state;
    const en = obj.enabled;
    el.className = 'ch-badge ' + (en === false || st === 'disabled'
      ? 'disabled' : st === 'connected' ? 'connected' : 'disconnected');
    el.textContent = en === false ? 'off' : st || 'disconnected';
  });
}
