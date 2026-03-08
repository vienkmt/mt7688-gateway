// --- Toolbox page ---

let _tbEl = null, _tbR = 0;

function renderToolbox() {
  if (!S.toolbox) S.toolbox = { tool: 'ping', target: '', lines: [], running: false };
  const tb = S.toolbox;

  const sel = h('select', {
    style: 'padding:8px 12px;border:1px solid #334155;background:#0f172a;color:#e2e8f0;border-radius:6px;font-size:.85rem',
    onchange: e => { tb.tool = e.target.value }
  },
    ...['ping', 'traceroute', 'nslookup'].map(t =>
      h('option', { value: t, ...(tb.tool === t ? { selected: '' } : {}) }, t)
    )
  );

  const ti = h('input', {
    type: 'text',
    value: tb.target,
    placeholder: 'hostname or IP',
    style: 'flex:1;padding:8px 12px;border:1px solid #334155;background:#0f172a;color:#e2e8f0;border-radius:6px;font-size:.85rem',
    oninput: e => { tb.target = e.target.value },
    onkeydown: e => { if (e.key === 'Enter' && !tb.running) tbRun() }
  });

  const run = h('button', {
    cls: 'save-btn',
    style: 'margin:0;padding:6px 16px;font-size:.8rem;' + (tb.running ? 'opacity:.5' : ''),
    onclick: () => { if (!tb.running) tbRun() }
  }, tb.running ? 'Running...' : 'Run');

  const stop = tb.running
    ? h('button', {
        style: 'padding:6px 16px;background:#dc2626;color:white;border:none;border-radius:6px;cursor:pointer;font-size:.8rem;font-weight:700',
        onclick: tbStop
      }, 'Stop')
    : null;

  const clr = h('button', {
    style: 'padding:6px 16px;background:#334155;color:#94a3b8;border:1px solid #475569;border-radius:6px;cursor:pointer;font-size:.8rem',
    onclick: () => { tb.lines = []; _tbEl = null; _tbR = 0; render() }
  }, 'Clear');

  const bar = h('div', {
    cls: 'diag-bar',
    style: 'display:flex;gap:8px;align-items:center;flex-wrap:wrap;margin-bottom:8px'
  },
    sel, ti,
    h('div', { cls: 'diag-bar-btns', style: 'display:flex;gap:8px' },
      run, ...(stop ? [stop] : []), clr)
  );

  _tbEl = h('div', { cls: 'stream', style: 'flex:1;overflow-y:auto;min-height:0' });
  tb.lines.forEach(l => { _tbEl.append(h('div', {}, l)) });
  _tbR = tb.lines.length;
  _tbEl.scrollTop = _tbEl.scrollHeight;

  if (S._diagOpen == null) S._diagOpen = true;

  const diagArrow = h('span', {
    cls: 'arrow',
    style: 'transition:transform .2s;display:inline-block;' + (S._diagOpen ? 'transform:rotate(90deg)' : '')
  }, '\u25B6');

  const diagTog = h('button', {
    cls: 'collapse-btn',
    onclick: () => { S._diagOpen = !S._diagOpen; render() }
  }, diagArrow);

  const diagBody = S._diagOpen ? [bar, _tbEl] : [];

  const diagCard = h('div', {
    cls: 'card',
    style: 'display:flex;flex-direction:column;' + (S._diagOpen ? 'flex:1;min-height:0;overflow:hidden' : '')
  },
    h('h3', {}, 'Network Diagnostics', diagTog),
    ...diagBody
  );

  const syslogCard = renderSyslog();

  return h('div', {
    style: 'display:flex;flex-direction:column;height:calc(100vh - 120px)'
  }, diagCard, syslogCard);
}

function renderToolboxStream() {
  if (!_tbEl || S.page !== 'toolbox') {
    render();
    return;
  }
  const tb = S.toolbox;
  while (_tbR < tb.lines.length) {
    _tbEl.append(h('div', {}, tb.lines[_tbR]));
    _tbR++;
  }
  _tbEl.scrollTop = _tbEl.scrollHeight;
  if (!tb.running) render();
}

function isSafeTarget(s) {
  return s.length > 0 && s.length <= 253 && !/^-/.test(s) && /^[a-zA-Z0-9.\-:]+$/.test(s);
}

async function tbRun() {
  const tb = S.toolbox;
  if (!tb.target.trim()) return;
  if (!isSafeTarget(tb.target.trim())) {
    toast('Target không hợp lệ (chỉ hostname/IP)', 'err');
    return;
  }
  tb.lines = [];
  _tbEl = null;
  _tbR = 0;
  tb.running = true;
  render();
  try {
    const r = await fetch('/api/toolbox/run', {
      method: 'POST',
      body: JSON.stringify({ tool: tb.tool, target: tb.target })
    });
    if (!r.ok) {
      const d = await r.json();
      tb.lines.push('Error: ' + (d.error || 'failed'));
      tb.running = false;
      render();
    }
  } catch (e) {
    tb.lines.push('Error: ' + e.message);
    tb.running = false;
    render();
  }
}

async function tbStop() {
  try { await fetch('/api/toolbox/stop', { method: 'POST' }) } catch (_) {}
}

// --- Syslog viewer ---

let _slEl = null, _slR = 0;

function renderSyslog() {
  if (!S.syslog) S.syslog = { lines: [], running: false, filter: '' };
  const sl = S.syslog;

  const fi = h('input', {
    type: 'text',
    value: sl.filter,
    placeholder: 'Filter (ví dụ: MQTT, HTTP, error...)',
    style: 'flex:1;padding:8px 12px;border:1px solid #334155;background:#0f172a;color:#e2e8f0;border-radius:6px;font-size:.85rem',
    oninput: e => { sl.filter = e.target.value; renderSyslogFiltered() }
  });

  const toggle = h('button', {
    style: 'padding:6px 16px;background:' + (sl.running ? '#dc2626' : '#2563eb')
      + ';color:white;border:none;border-radius:6px;cursor:pointer;font-weight:700;font-size:.8rem',
    onclick: () => { sl.running ? slStop() : slStart() }
  }, sl.running ? 'Stop' : 'Start');

  const clr = h('button', {
    style: 'padding:6px 16px;background:#334155;color:#94a3b8;border:1px solid #475569;border-radius:6px;cursor:pointer;font-size:.8rem',
    onclick: () => { sl.lines = []; _slEl = null; _slR = 0; render() }
  }, 'Clear');

  const bar = h('div', {
    cls: 'sl-bar',
    style: 'display:flex;gap:8px;align-items:center;flex-wrap:wrap;margin-bottom:8px'
  },
    fi,
    h('div', { cls: 'sl-bar-btns', style: 'display:flex;gap:8px' }, toggle, clr)
  );

  _slEl = h('div', { cls: 'stream', style: 'flex:1;overflow-y:auto;min-height:0' });

  const filtered = sl.filter
    ? sl.lines.filter(l => l.text.toLowerCase().includes(sl.filter.toLowerCase()))
    : sl.lines;
  filtered.forEach(l => { _slEl.append(slRow(l)) });
  _slR = sl.lines.length;
  _slEl.scrollTop = _slEl.scrollHeight;

  return h('div', {
    cls: 'card',
    style: 'display:flex;flex-direction:column;flex:2;min-height:0;overflow:hidden'
  },
    h('h3', {}, 'System Log'),
    bar,
    _slEl
  );
}

function slRow(l) {
  const d = h('div', { style: 'padding:2px 0;border-bottom:1px solid #1e293b' });
  if (l.level === 'warn') d.style.color = '#facc15';
  else if (l.level === 'err') d.style.color = '#f87171';
  d.textContent = l.text;
  return d;
}

function renderSyslogStream() {
  if (!_slEl || S.page !== 'toolbox') return;
  const sl = S.syslog;
  while (_slR < sl.lines.length) {
    const l = sl.lines[_slR];
    if (!sl.filter || l.text.toLowerCase().includes(sl.filter.toLowerCase())) {
      _slEl.append(slRow(l));
    }
    _slR++;
  }
  _slEl.scrollTop = _slEl.scrollHeight;
}

function renderSyslogFiltered() {
  if (!_slEl) return;
  const sl = S.syslog;
  _slEl.innerHTML = '';
  const filtered = sl.filter
    ? sl.lines.filter(l => l.text.toLowerCase().includes(sl.filter.toLowerCase()))
    : sl.lines;
  filtered.forEach(l => { _slEl.append(slRow(l)) });
  _slEl.scrollTop = _slEl.scrollHeight;
}

async function slStart() {
  const sl = S.syslog;
  sl.running = true;
  render();
  try {
    const r = await fetch('/api/syslog/start', { method: 'POST' });
    if (!r.ok) {
      sl.running = false;
      render();
    }
  } catch (_) {
    sl.running = false;
    render();
  }
}

async function slStop() {
  try { await fetch('/api/syslog/stop', { method: 'POST' }) } catch (_) {}
  S.syslog.running = false;
  render();
}
