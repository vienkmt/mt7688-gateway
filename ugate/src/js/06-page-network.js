// --- Network page ---

function renderNetwork() {
  if (!S.wifi.status) loadWifiStatus().then(render);
  checkPendingChanges();
  const banner = renderPendingBanner();
  return h('div', {}, banner, renderWifi(), renderLanWan(), renderNtp());
}

function renderWifi() {
  const w = S.wifi;
  const ws = w.status || {};
  const mode = ws.mode || 'sta_ap';
  if (!w._mode) w._mode = mode;

  // Mode selector
  const modeOpts = [
    ['sta', 'STA (Client)'],
    ['ap', 'AP (Phát WiFi)'],
    ['sta_ap', 'STA + AP'],
    ['off', 'Tắt WiFi']
  ];
  const modeSelect = h('select', {
    style: 'padding:4px 8px;border:1px solid #334155;background:#0f172a;color:#e2e8f0;border-radius:4px;font-size:.8rem',
    onchange: e => { w._mode = e.target.value; render() }
  },
    ...modeOpts.map(([v, t]) =>
      h('option', { value: v, ...(w._mode === v ? { selected: '' } : {}) }, t)
    )
  );

  // STA section — input SSID gõ tay, quét WiFi chỉ hỗ trợ điền
  const sta = ws.sta || {};
  if (w._staSsid == null) w._staSsid = sta.config_ssid || '';
  if (w._staPwd == null) w._staPwd = sta.config_key || '';
  if (!w._staEnc) w._staEnc = sta.config_enc || 'psk2';

  const secStyle = 'background:#0f172a;border:1px solid #334155;border-radius:8px;padding:12px;margin-bottom:12px';

  const pwdInput = (val, onset, ph) => {
    const wrap = h('div', { style: 'position:relative;width:100%' });
    const input = h('input', {
      type: 'password',
      value: val,
      placeholder: ph || '',
      style: 'width:100%;padding:6px 32px 6px 10px;border:1px solid #334155;background:#0f172a;color:#e2e8f0;border-radius:4px;font-size:.85rem',
      oninput: e => onset(e.target.value)
    });
    const eye = h('span', {
      style: 'position:absolute;right:8px;top:50%;transform:translateY(-50%);cursor:pointer;color:#64748b;font-size:.8rem;user-select:none',
      onclick: () => {
        const show = input.type === 'password';
        input.type = show ? 'text' : 'password';
        eye.textContent = show ? '\u25E1' : '\u25C9';
      }
    }, '\u25C9');
    wrap.append(input, eye);
    return wrap;
  };

  const staSection = (w._mode === 'sta' || w._mode === 'sta_ap')
    ? h('div', { style: secStyle },
        h('div', {
          style: 'display:flex;justify-content:space-between;align-items:center;margin-bottom:6px'
        },
          h('div', {
            style: 'font-weight:600;color:#38bdf8;font-size:.85rem;white-space:nowrap'
          }, 'STA — Kết nối WiFi'),
          h('button', {
            cls: 'scan-btn',
            style: 'margin:0;padding:3px 8px;font-size:.7rem;background:#334155;color:#94a3b8;border:1px solid #475569;border-radius:4px;cursor:pointer;white-space:nowrap;flex-shrink:0',
            onclick: scanWifi
          }, 'Quét')
        ),
        sta.connected
          ? h('div', { cls: 'cf', style: 'margin-bottom:8px' },
              lbl('Trạng thái'), h('span', { style: 'color:#22c55e' }, 'Đã kết nối'),
              lbl('Tín hiệu'),
              h('span', { style: 'display:flex;align-items:center;gap:6px' },
                signalBars(sta.signal), sta.signal + ' dBm'),
              lbl('IP'), h('span', {}, sta.ip || '-')
            )
          : h('div', {
              style: 'margin-bottom:8px;padding:6px 10px;background:#7f1d1d22;border:1px solid #7f1d1d;border-radius:6px;font-size:.8rem;color:#fca5a5'
            },
              sta.config_ssid
                ? 'Không kết nối được "' + sta.config_ssid + '" — kiểm tra SSID/mật khẩu hoặc router'
                : 'Chưa cấu hình WiFi'
            ),
        h('div', { cls: 'cf' },
          lbl('SSID'),
          h('input', {
            type: 'text',
            id: 'sta-ssid',
            value: w._staSsid,
            placeholder: 'Nhập hoặc quét WiFi',
            oninput: e => { w._staSsid = e.target.value }
          }),
          lbl('Mật khẩu'),
          pwdInput(w._staPwd, v => { w._staPwd = v }, 'Để trống nếu không cần')
        )
      )
    : h('div', {});

  // AP section
  const ap = ws.ap || {};
  if (!w._apSsid && w._apSsid !== '') w._apSsid = ap.ssid || '';
  if (!w._apKey && w._apKey !== '') w._apKey = ap.key || '';
  if (!w._apEnc) w._apEnc = ap.encryption || 'psk2';
  if (!w._apCh) w._apCh = ap.channel || 'auto';

  const apSection = (w._mode === 'ap' || w._mode === 'sta_ap')
    ? h('div', { style: secStyle },
        h('div', {
          style: 'font-weight:600;color:#22c55e;margin-bottom:6px;font-size:.85rem'
        }, 'AP — Phát WiFi' + (ap.active ? ' (đang phát)' : '')),
        h('div', { cls: 'cf' },
          lbl('Tên WiFi'),
          h('input', {
            type: 'text',
            value: w._apSsid,
            oninput: e => { w._apSsid = e.target.value }
          }),
          lbl('Mật khẩu'),
          pwdInput(w._apKey, v => { w._apKey = v }, 'Để trống = WiFi mở'),
          ...(w._mode === 'ap'
            ? [
                lbl('Kênh'),
                h('select', {
                  style: 'padding:6px 10px;border:1px solid #334155;background:#0f172a;color:#e2e8f0;border-radius:4px;font-size:.85rem;width:100%',
                  onchange: e => { w._apCh = e.target.value }
                },
                  ...[ ['auto', 'Tự động'],
                    ...[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11].map(c => [String(c), 'Kênh ' + c])
                  ].map(([v, t]) =>
                    h('option', { value: v, ...(w._apCh === v ? { selected: '' } : {}) }, t)
                  )
                )
              ]
            : [
                h('div', {
                  style: 'grid-column:1/-1;color:#64748b;font-size:.75rem;font-style:italic'
                }, 'Kênh AP tự theo STA khi ở chế độ STA+AP')
              ]
          )
        )
      )
    : h('div', {});

  // 2 nút: Lưu nháp (RAM) + Áp dụng (commit flash + wifi reload)
  const btnRow = h('div', { style: 'display:flex;justify-content:center;margin-top:8px' },
    h('button', {
      cls: 'save-btn',
      onclick: () => saveWifiMode(w._mode, w)
    }, 'Lưu nháp')
  );

  const offNotice = w._mode === 'off'
    ? h('div', {
        style: 'color:#94a3b8;text-align:center;padding:16px;font-size:.85rem'
      }, 'WiFi đã tắt — chỉ dùng Ethernet')
    : h('div', {});

  const header = h('h3', {}, 'WiFi',
    h('div', { style: 'display:flex;align-items:center;gap:8px' }, modeSelect)
  );

  return h('div', { cls: 'card' }, header, offNotice, staSection, apSection, btnRow);
}

function showWifiModal() {
  const existing = document.getElementById('wifi-modal');
  if (existing) existing.remove();

  const overlay = h('div', {
    cls: 'modal-overlay',
    id: 'wifi-modal',
    onclick: e => { if (e.target === overlay) overlay.remove() }
  });
  const modal = h('div', { cls: 'modal', style: 'position:relative' });
  const content = h('div', { id: 'wifi-modal-content' });

  modal.append(
    h('h3', {}, 'Quét WiFi'),
    h('button', { cls: 'modal-close', onclick: () => overlay.remove() }, '\u2715'),
    content
  );
  overlay.append(modal);
  document.body.append(overlay);
  return content;
}

function updateWifiModal(content, networks) {
  content.innerHTML = '';
  if (!networks.length) {
    content.append(h('div', {
      style: 'color:#94a3b8;text-align:center;padding:20px'
    }, 'Không tìm thấy WiFi nào'));
    return;
  }

  networks.sort((a, b) => b.signal - a.signal);

  networks.forEach(n => {
    content.append(
      h('div', {
        cls: 'wifi-item',
        onclick: () => {
          S.wifi._staSsid = n.ssid;
          S.wifi._staPwd = '';
          // Auto-detect encryption from scan
          const enc = n.encryption || '';
          if (enc.includes('WPA3') || enc.includes('SAE')) S.wifi._staEnc = 'sae';
          else if (enc.includes('mixed')) S.wifi._staEnc = 'psk-mixed';
          else if (enc.includes('WPA2') || enc.includes('PSK')) S.wifi._staEnc = 'psk2';
          else if (enc === 'none' || enc === 'Open') S.wifi._staEnc = 'none';
          document.getElementById('wifi-modal').remove();
          render();
        }
      },
        h('div', {},
          h('div', { style: 'color:#e2e8f0;font-weight:500' }, n.ssid),
          h('div', { style: 'color:#64748b;font-size:.75rem' }, n.encryption)
        ),
        h('div', { style: 'display:flex;align-items:center;gap:8px' },
          signalBars(n.signal),
          h('span', { style: 'color:#64748b;font-size:.75rem' }, n.signal + ' dBm')
        )
      )
    );
  });
}

function renderLanWan() {
  if (!S.net) {
    loadNetwork();
    return h('div', { cls: 'card' }, 'Đang tải...');
  }
  const n = S.net;
  const w = n.wan;
  if (!w._dns) w._dns = (w.dns || []).join(',');

  // Prefill static fields from current DHCP values when switching
  if (w.proto === 'static' && !w.ipaddr && w._dhcp_ip) {
    w.ipaddr = w._dhcp_ip;
    w.netmask = w._dhcp_mask || '255.255.255.0';
    w.gateway = w._dhcp_gw || '';
    w._dns = w._dhcp_dns || '';
  }

  const fields = [
    lbl('Giao thức'),
    slct(w, 'proto', [['dhcp', 'DHCP'], ['static', 'Static IP']], true, () => render())
  ];
  if (w.proto === 'static') {
    fields.push(
      lbl('IP'), inp('text', w, 'ipaddr', true),
      lbl('Netmask'), inp('text', w, 'netmask'),
      lbl('Gateway'), inp('text', w, 'gateway'),
      lbl('DNS'), inp('text', w, '_dns', true)
    );
  }

  return h('div', { cls: 'card' },
    h('h3', {}, 'ETH WAN',
      h('span', {
        style: 'font-size:.7rem;color:#64748b;font-weight:400;margin-left:8px'
      }, 'eth0.2')
    ),
    h('div', { cls: 'cf' }, ...fields),
    h('button', { cls: 'save-btn', onclick: saveNetwork }, 'Lưu nháp')
  );
}

function renderNtp() {
  if (!S.ntp) {
    loadNtp();
    return h('div', { cls: 'card' }, 'Đang tải...');
  }
  const n = S.ntp;
  if (!n._srvs) n._srvs = [...(n.servers || [])];
  if (!n._srvs.length) n._srvs = [''];

  const hdr = h('h3', { style: 'display:flex;align-items:center;gap:10px' }, 'NTP',
    h('label', { cls: 'sw' },
      h('input', {
        type: 'checkbox',
        ...(n.enabled ? { checked: '' } : {}),
        onchange: e => { n.enabled = e.target.checked }
      }),
      h('span', { cls: 'sl' })
    )
  );

  const fields = [
    lbl('Timezone'),
    slct(n, 'timezone', [
      ['ICT-7', 'Asia/Ho_Chi_Minh (UTC+7)'],
      ['CST-8', 'Asia/Shanghai (UTC+8)'],
      ['JST-9', 'Asia/Tokyo (UTC+9)'],
      ['WIB-7', 'Asia/Jakarta (UTC+7)'],
      ['SGT-8', 'Asia/Singapore (UTC+8)'],
      ['<+09>-9', 'Asia/Seoul (UTC+9)'],
      ['IST-5:30', 'Asia/Kolkata (UTC+5:30)'],
      ['GST-4', 'Asia/Dubai (UTC+4)'],
      ['UTC0', 'UTC'],
      ['GMT0', 'Europe/London (GMT)'],
      ['CET-1', 'Europe/Paris (UTC+1)'],
      ['EET-2', 'Europe/Bucharest (UTC+2)'],
      ['EST5EDT', 'US/Eastern'],
      ['CST6CDT', 'US/Central'],
      ['MST7MDT', 'US/Mountain'],
      ['PST8PDT', 'US/Pacific'],
      ['AEST-10', 'Australia/Sydney (UTC+10)']
    ])
  ];

  // Dynamic server inputs, 2 per row
  n._srvs.forEach((sv, i) => {
    fields.push(
      lbl('Server ' + (i + 1)),
      h('input', {
        type: 'text',
        value: sv,
        placeholder: 'pool.ntp.org',
        oninput: e => { n._srvs[i] = e.target.value }
      })
    );
  });

  // Add/remove buttons inline
  const addRm = h('div', { style: 'grid-column:1/-1;display:flex;gap:8px' });
  addRm.append(
    h('button', {
      style: 'padding:3px 10px;background:#1e3a5f;color:#93c5fd;border:1px solid #334155;border-radius:4px;cursor:pointer;font-size:.75rem',
      onclick: () => { n._srvs.push(''); render() }
    }, '+ Thêm')
  );
  if (n._srvs.length > 1) {
    addRm.append(
      h('button', {
        style: 'padding:3px 10px;background:#991b1b;color:#fca5a5;border:none;border-radius:4px;cursor:pointer;font-size:.75rem',
        onclick: () => { n._srvs.pop(); render() }
      }, '- Xoá cuối')
    );
  }
  fields.push(addRm);

  const btns = h('div', {
    cls: 'ntp-btns',
    style: 'display:flex;gap:8px;justify-content:center;margin-top:8px'
  });
  btns.append(
    h('button', {
      cls: 'save-btn',
      style: 'margin:0;background:#334155',
      onclick: syncNtp
    }, 'Đồng bộ ngay')
  );
  btns.append(
    h('button', {
      cls: 'save-btn',
      style: 'margin:0',
      onclick: saveNtp
    }, 'Lưu NTP')
  );

  return h('div', { cls: 'card' }, hdr, h('div', { cls: 'cf' }, ...fields), btns);
}

// Network API calls

async function scanWifi() {
  const content = showWifiModal();
  content.append(
    h('div', { cls: 'spinner' }),
    h('div', { style: 'text-align:center;color:#94a3b8;margin-top:8px' }, 'Đang quét...')
  );
  try {
    const r = await fetch('/api/wifi/scan');
    if (r.ok) {
      const d = await r.json();
      S.wifi.networks = d.networks || [];
      updateWifiModal(content, S.wifi.networks);
    } else {
      content.innerHTML = '';
      content.append(h('div', {
        style: 'color:#ef4444;text-align:center;padding:20px'
      }, 'Lỗi quét WiFi'));
    }
  } catch (_) {
    content.innerHTML = '';
    content.append(h('div', {
      style: 'color:#ef4444;text-align:center;padding:20px'
    }, 'Lỗi kết nối'));
  }
}

async function loadWifiStatus() {
  try {
    const r = await fetch('/api/wifi/status');
    if (r.ok) {
      S.wifi.status = await r.json();
      S.wifi._mode = null;
      S.wifi._staSsid = null;
      S.wifi._staPwd = null;
      S.wifi._staEnc = null;
      S.wifi._apSsid = null;
      S.wifi._apKey = null;
      S.wifi._apEnc = null;
      S.wifi._apCh = null;
      return;
    }
  } catch (_) {}

  if (!S.wifi.status) {
    S.wifi.status = {
      mode: 'sta_ap',
      sta: { connected: false, ssid: '', config_ssid: '', signal: 0, ip: '' },
      ap: { active: false, ssid: '', encryption: '', key: '', channel: '' }
    };
  }
}

async function connectWifi(ssid, pwd) {
  try {
    const r = await fetch('/api/wifi/connect', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ ssid, password: pwd, encryption: 'psk2' })
    });
    if (r.ok) {
      toast('Đang kết nối WiFi...', 'ok');
      S.wifi._sel = null;
      setTimeout(() => { loadWifiStatus().then(render) }, 5000);
    } else {
      toast('Lỗi kết nối', 'err');
    }
  } catch (_) {
    toast('Lỗi kết nối', 'err');
  }
}

async function disconnectWifi() {
  try {
    await fetch('/api/wifi/disconnect', { method: 'POST' });
    toast('Đang ngắt WiFi...', 'ok');
    setTimeout(() => { loadWifiStatus().then(render) }, 5000);
  } catch (_) {}
}

async function saveWifiMode(mode, w) {
  const body = { mode };
  if (mode === 'sta' || mode === 'sta_ap') {
    if (w._staSsid) body.sta_ssid = w._staSsid;
    body.sta_password = w._staPwd || '';
  }
  if (mode === 'ap' || mode === 'sta_ap') {
    if (w._apSsid) body.ap_ssid = w._apSsid;
    body.ap_password = w._apKey || '';
    if (w._apCh) body.ap_channel = w._apCh;
  }
  try {
    const r = await fetch('/api/wifi/mode', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body)
    });
    if (r.ok) {
      toast('Đã lưu chế độ WiFi (draft)', 'ok');
      S.pendingChanges = true;
      setTimeout(() => { loadWifiStatus().then(render) }, 2000);
    } else {
      toast('Lỗi lưu chế độ WiFi', 'err');
    }
  } catch (_) {
    toast('Lỗi kết nối', 'err');
  }
}

async function loadNetwork() {
  try {
    const r = await fetch('/api/network');
    if (r.ok) {
      S.net = await r.json();
      render();
      return;
    }
  } catch (_) {}
  S.net = {
    lan: { name: 'lan', proto: 'dhcp', ipaddr: '', netmask: '255.255.255.0', gateway: '', dns: [] },
    wan: { name: 'wan', proto: 'dhcp', ipaddr: '', netmask: '255.255.255.0', gateway: '', dns: [] }
  };
  render();
}

async function saveNetwork() {
  const w = S.net.wan;
  const body = { interface: 'wan', proto: w.proto };
  if (w.proto === 'static') {
    body.ipaddr = w.ipaddr;
    body.netmask = w.netmask;
    body.gateway = w.gateway;
    body.dns = w._dns || '';
  }
  try {
    const r = await fetch('/api/network', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body)
    });
    if (!r.ok) {
      toast('Lỗi lưu WAN', 'err');
      return;
    }
  } catch (_) {
    toast('Lỗi kết nối', 'err');
    return;
  }
  S.pendingChanges = true;
  toast('Đã lưu nháp (RAM)', 'ok');
  render();
}

async function loadNtp() {
  try {
    const r = await fetch('/api/ntp');
    if (r.ok) {
      S.ntp = await r.json();
      render();
      return;
    }
  } catch (_) {}
  S.ntp = { enabled: true, servers: [], timezone: 'ICT-7', zonename: 'Asia/Ho_Chi_Minh' };
  render();
}

async function saveNtp() {
  const n = S.ntp;
  const srvs = (n._srvs || []).filter(s => s.trim());
  const body = {
    enabled: !!n.enabled,
    timezone: n.timezone || '',
    zonename: n.zonename || '',
    servers: srvs.join(',')
  };
  try {
    const r = await fetch('/api/ntp', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(body)
    });
    if (r.ok) toast('Đã lưu NTP', 'ok');
    else toast('Lưu NTP lỗi', 'err');
  } catch (_) {
    toast('Lỗi kết nối', 'err');
  }
}

async function syncNtp() {
  try {
    const r = await fetch('/api/ntp/sync', { method: 'POST' });
    if (r.ok) {
      const d = await r.json();
      toast('Đã đồng bộ (' + d.method + ')', 'ok');
    } else {
      toast('Lỗi sync', 'err');
    }
  } catch (_) {
    toast('Lỗi kết nối', 'err');
  }
}
