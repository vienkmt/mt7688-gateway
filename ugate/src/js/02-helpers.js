// --- UI helpers ---

function lbl(text) {
  return h('span', { cls: 'lbl' }, text);
}

function gap() {
  return h('div', { cls: 'gap' });
}

function helpLink(text, fn) {
  return h('a', {
    href: '#',
    cls: 'help-link',
    onclick: e => { e.preventDefault(); fn() }
  }, text);
}

function inp(type, obj, key, full) {
  return h('input', {
    type,
    cls: full ? 'full' : '',
    value: type === 'number' ? (obj[key] || 0) : (obj[key] || ''),
    oninput: e => {
      obj[key] = type === 'number' ? +e.target.value : e.target.value;
    }
  });
}

function slct(obj, key, opts, full, cb) {
  const s = h('select', {
    cls: full ? 'full' : '',
    onchange: e => {
      obj[key] = e.target.value;
      if (cb) cb();
    }
  });
  opts.forEach(([v, t]) => {
    const o = h('option', { value: v }, t);
    if ('' + obj[key] === v) o.selected = true;
    s.append(o);
  });
  return s;
}

function swTog(obj, key) {
  return h('label', { cls: 'sw' },
    h('input', {
      type: 'checkbox',
      ...(obj[key] ? { checked: '' } : {}),
      onchange: e => { obj[key] = e.target.checked }
    }),
    h('span', { cls: 'sl' })
  );
}

function chHdr(title, obj) {
  const tog = h('input', {
    type: 'checkbox',
    ...(obj.enabled ? { checked: '' } : {}),
    onchange: e => { obj.enabled = e.target.checked; render() }
  });
  return h('h3', {}, title,
    h('label', { cls: 'sw' }, tog, h('span', { cls: 'sl' }))
  );
}

function pbar(id, label, val, max, color, text) {
  const pct = max > 0 ? Math.min(100, val / max * 100) : 0;
  // Reuse existing fill element for smooth transition
  const existing = document.querySelector('[data-pbar="' + id + '"]');
  if (existing) {
    existing.style.width = pct + '%';
    existing.style.background = color;
    const sp = existing.closest('.pbar-row').querySelector('span');
    if (sp) sp.textContent = text;
    return existing.closest('.pbar-row');
  }
  const fill = h('div', {
    cls: 'fill',
    'data-pbar': id,
    style: 'width:0%;background:' + color
  });
  setTimeout(() => { fill.style.width = pct + '%' }, 20);
  return h('div', { cls: 'pbar-row' },
    h('label', {}, label),
    h('div', { cls: 'pbar' }, fill),
    h('span', {}, text)
  );
}

function chBadge(state, enabled) {
  if (enabled === false) return h('span', { cls: 'ch-badge disabled' }, 'off');
  if (state === 'connected') return h('span', { cls: 'ch-badge connected' }, 'connected');
  if (state === 'waiting') return h('span', { cls: 'ch-badge disconnected' }, 'waiting');
  if (state === 'disabled') return h('span', { cls: 'ch-badge disabled' }, 'off');
  return h('span', { cls: 'ch-badge disconnected' }, state || 'disconnected');
}

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

function signalBars(dbm) {
  const strength = dbm > -50 ? 4 : dbm > -60 ? 3 : dbm > -70 ? 2 : 1;
  return h('span', { cls: 'wifi-signal' },
    ...[6, 10, 14, 18].map((ht, i) =>
      h('i', {
        style: 'height:' + ht + 'px',
        cls: i < strength ? 'active' : ''
      })
    )
  );
}

// Interface name mapping
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

function ifaceBadge(raw) {
  const name = ifaceName(raw);
  const color = name.includes('ETH') ? '#3b82f6'
    : name.includes('WiFi WAN') ? '#f59e0b'
    : name.includes('AP') ? '#22c55e'
    : '#64748b';
  return h('span', {
    style: 'display:inline-flex;align-items:center;gap:4px;padding:2px 8px;background:'
      + color + '22;color:' + color
      + ';border-radius:4px;font-size:.75rem;font-weight:600'
  }, name);
}

function renderPendingBanner() {
  if (!S.pendingChanges) return h('div', {});
  return h('div', {
    cls: 'pending-banner',
    style: 'background:#92400e;border:1px solid #f59e0b;border-radius:8px;padding:12px 16px;margin-bottom:12px;display:flex;justify-content:space-between;align-items:center'
  },
    h('span', {
      style: 'color:#fbbf24;font-weight:600;font-size:.85rem'
    }, '⚠ Có thay đổi chưa lưu vào flash'),
    h('div', {
      cls: 'pending-btns',
      style: 'display:flex;gap:8px'
    },
      h('button', {
        style: 'padding:6px 16px;background:#334155;color:#94a3b8;border:1px solid #475569;border-radius:6px;cursor:pointer;font-size:.8rem',
        onclick: revertChanges
      }, 'Huỷ nháp'),
      h('button', {
        style: 'padding:6px 16px;background:#16a34a;color:white;border:none;border-radius:6px;cursor:pointer;font-weight:700;font-size:.8rem',
        onclick: applyChanges
      }, 'Áp dụng & Lưu flash')
    )
  );
}
