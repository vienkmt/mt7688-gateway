// --- Routing page ---

function renderRouting() {
  checkPendingChanges();
  const banner = renderPendingBanner();
  return h('div', {}, banner, renderMetrics(), renderRoutes());
}

function renderMetrics() {
  if (!S.wanIfaces) {
    loadWanIfaces();
    return h('div', { cls: 'card' }, 'Đang tải...');
  }
  const ifaces = S.wanIfaces;
  if (!ifaces.length) {
    return h('div', { cls: 'card' },
      h('h3', {}, 'Ưu tiên mạng'),
      h('div', { style: 'color:#64748b;font-size:.85rem' }, 'Không có WAN interface nào')
    );
  }

  // Init editable metric values
  ifaces.forEach(ifc => {
    if (ifc._metric == null) ifc._metric = ifc.uci_metric || ifc.metric || '0';
  });

  const fields = [];
  ifaces.forEach(ifc => {
    fields.push(
      lbl(ifc.label),
      h('div', { style: 'display:flex;align-items:center;gap:8px' },
        h('input', {
          type: 'number',
          value: ifc._metric,
          style: 'width:80px;padding:6px 10px;border:1px solid #334155;background:#0f172a;color:#e2e8f0;border-radius:4px;font-size:.85rem',
          oninput: e => { ifc._metric = e.target.value }
        }),
        h('span', { style: 'color:#64748b;font-size:.75rem' },
          'via ' + ifc.gateway + ' (' + ifc.dev + ')')
      )
    );
  });

  return h('div', { cls: 'card' },
    h('h3', {}, 'Ưu tiên mạng',
      h('span', {
        style: 'font-size:.7rem;color:#64748b;font-weight:400;margin-left:8px'
      }, 'Metric thấp = ưu tiên cao')
    ),
    h('div', { cls: 'cf' }, ...fields),
    h('button', { cls: 'save-btn', onclick: saveMetrics }, 'Lưu nháp')
  );
}

function renderRoutes() {
  if (!S.routes) {
    loadRoutes();
    return h('div', { cls: 'card' }, 'Đang tải...');
  }
  const routes = S.routes.routes || [];
  const r = S.newRoute;

  // Route table
  const thead = h('tr', {},
    ...[
      ['Đích', '25%'], ['Gateway', '20%'], ['Interface', '20%'],
      ['Metric', '10%'], ['Scope', '10%']
    ].map(([t, w]) =>
      h('th', {
        style: 'text-align:left;padding:6px 8px;color:#64748b;font-size:.75rem;font-weight:600;border-bottom:1px solid #334155;width:' + w
      }, t)
    )
  );

  const tbody = routes.map(rt =>
    h('tr', {},
      h('td', { style: 'padding:6px 8px;font-size:.8rem;color:#e2e8f0;font-family:monospace' }, rt.dest),
      h('td', { style: 'padding:6px 8px;font-size:.8rem;color:#94a3b8;font-family:monospace' }, rt.via),
      h('td', { style: 'padding:6px 8px' }, ifaceBadge(rt.dev)),
      h('td', { style: 'padding:6px 8px;font-size:.8rem;color:#94a3b8;text-align:center' }, rt.metric),
      h('td', { style: 'padding:6px 8px;font-size:.8rem;color:#64748b' }, rt.scope)
    )
  );

  const tableEl = routes.length
    ? h('table', { style: 'width:100%;border-collapse:collapse;margin-bottom:12px' },
        h('thead', {}, thead),
        h('tbody', {}, ...tbody))
    : h('div', { style: 'color:#64748b;font-size:.85rem;margin-bottom:12px' }, 'Chưa có route nào');

  const table = routes.length ? h('div', { cls: 'route-wrap' }, tableEl) : tableEl;

  // Add route form
  return h('div', { cls: 'card' },
    h('h3', {}, 'Routing'),
    h('div', {
      style: 'font-weight:600;color:#38bdf8;margin-bottom:6px;font-size:.85rem'
    }, 'Bảng định tuyến'),
    table,
    h('div', {
      style: 'font-weight:600;color:#38bdf8;margin-bottom:6px;font-size:.85rem'
    }, 'Thêm static route'),
    h('div', { cls: 'cf' },
      lbl('Tên'), inp('text', r, 'name'),
      lbl('Interface'), slct(r, 'interface', [['wan', 'ETH WAN'], ['wwan', 'WiFi WAN']]),
      lbl('Target'), inp('text', r, 'target'),
      lbl('Netmask'), inp('text', r, 'netmask'),
      lbl('Gateway'), inp('text', r, 'gateway', true)
    ),
    h('button', { cls: 'save-btn', onclick: addRoute }, 'Thêm route')
  );
}

async function loadWanIfaces() {
  try {
    const r = await fetch('/api/wan/discover');
    if (r.ok) {
      const d = await r.json();
      S.wanIfaces = d.interfaces || [];
      render();
      return;
    }
  } catch (_) {}
  S.wanIfaces = [];
  render();
}

async function loadRoutes() {
  try {
    const r = await fetch('/api/routes');
    if (r.ok) {
      S.routes = await r.json();
      render();
      return;
    }
  } catch (_) {}
  S.routes = { current_table: '' };
  render();
}

async function addRoute() {
  const r = S.newRoute;
  if (!r.name || !r.target || !r.gateway) {
    toast('Điền đầy đủ thông tin', 'err');
    return;
  }
  try {
    const res = await fetch('/api/routes', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(r)
    });
    if (res.ok) {
      toast('Đã thêm route', 'ok');
      S.routes = null;
      S.newRoute = { name: '', target: '', netmask: '255.255.255.0', gateway: '', interface: 'wan' };
      render();
    } else {
      const d = await res.json();
      toast(d.error || 'Lỗi', 'err');
    }
  } catch (_) {
    toast('Lỗi kết nối', 'err');
  }
}

async function checkPendingChanges() {
  try {
    const r = await fetch('/api/network/changes');
    if (r.ok) {
      const d = await r.json();
      const had = S.pendingChanges;
      S.pendingChanges = !!d.pending;
      if (had !== S.pendingChanges) render();
    }
  } catch (_) {}
}

async function applyChanges() {
  try {
    const r = await fetch('/api/network/apply', { method: 'POST' });
    if (r.ok) {
      toast('Đã lưu vào flash, đang khởi động lại...', 'ok');
      S.pendingChanges = false;
      render();
    } else {
      toast('Lỗi áp dụng', 'err');
    }
  } catch (_) {
    toast('Lỗi kết nối', 'err');
  }
}

async function revertChanges() {
  try {
    const r = await fetch('/api/network/revert', { method: 'POST' });
    if (r.ok) {
      toast('Đã huỷ thay đổi', 'ok');
      S.pendingChanges = false;
      S.net = null;
      S.ntp = null;
      render();
    } else {
      toast('Lỗi huỷ', 'err');
    }
  } catch (_) {
    toast('Lỗi kết nối', 'err');
  }
}

async function saveMetrics() {
  if (!S.wanIfaces) return;
  for (const ifc of S.wanIfaces) {
    try {
      await fetch('/api/interface/metric', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ interface: ifc.uci, metric: +(ifc._metric || 0) })
      });
    } catch (_) {}
  }
  S.pendingChanges = true;
  toast('Đã lưu nháp (RAM). Ấn "Áp dụng" để ghi flash.', 'ok');
  render();
}
