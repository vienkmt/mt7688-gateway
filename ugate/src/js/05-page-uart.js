// --- UART page ---

function renderUart() {
  if (!S.config) {
    loadConfig();
    return h('div', { cls: 'card' }, 'Đang tải...');
  }
  const c = S.config;
  const fm = c.uart.frame_mode;

  const bauds = [
    ['9600', '9600'], ['19200', '19200'], ['38400', '38400'], ['57600', '57600'],
    ['115200', '115200'], ['230400', '230400'], ['460800', '460800'], ['921600', '921600']
  ];

  const fields = [
    lbl('Baudrate'), slct(c.uart, 'baudrate', bauds),
    lbl('Data Bits'), slct(c.uart, 'data_bits', [['7', '7'], ['8', '8']]),
    lbl('Parity'), slct(c.uart, 'parity', [['none', 'None'], ['even', 'Even'], ['odd', 'Odd']]),
    lbl('Stop Bits'), slct(c.uart, 'stop_bits', [['1', '1'], ['2', '2']]),
    lbl('Frame Mode'), slct(c.uart, 'frame_mode', [
      ['none', 'None (Gap)'], ['frame', 'Frame (Fixed)'], ['modbus', 'Modbus RTU']
    ], false, () => render())
  ];

  if (fm === 'none') {
    fields.push(lbl('Gap (ms)'), inp('number', c.uart, 'gap_ms'));
  }
  if (fm === 'frame') {
    fields.push(
      lbl('Frame Length'), inp('number', c.uart, 'frame_length'),
      lbl('Frame Timeout (ms)'), inp('number', c.uart, 'frame_timeout_ms')
    );
  }
  if (fm === 'modbus') {
    fields.push(lbl('Frame Timeout (ms)'), inp('number', c.uart, 'frame_timeout_ms'));
  }

  if (S._uartOpen == null) S._uartOpen = true;

  const arrow = h('span', {
    cls: 'arrow' + (S._uartOpen ? ' open' : ''),
    style: 'transition:transform .2s;display:inline-block;' + (S._uartOpen ? 'transform:rotate(90deg)' : '')
  }, '\u25B6');

  const togBtn = h('button', {
    cls: 'collapse-btn',
    onclick: () => { S._uartOpen = !S._uartOpen; render() }
  }, arrow);

  const body = S._uartOpen
    ? h('div', {},
        h('div', { cls: 'cf' }, ...fields),
        h('button', { cls: 'save-btn', onclick: saveConfig }, 'Lưu cấu hình'))
    : h('div', {});

  return h('div', { style: 'display:flex;flex-direction:column;height:calc(100vh - 120px)' },
    h('div', { cls: 'card', style: 'flex-shrink:0' },
      h('h3', {}, 'Cài đặt UART', togBtn),
      body
    ),
    renderData()
  );
}

function fmtRow(d) {
  if (!d || !d.type) return null;
  const isRx = d.dir !== 'tx';
  let content = '';

  if (d.hex) {
    if (S.hexView) {
      content = d.hex.match(/.{1,2}/g).join(' ').toUpperCase();
    } else {
      const bytes = d.hex.match(/.{1,2}/g) || [];
      content = bytes.map(b => {
        const c = parseInt(b, 16);
        return c >= 32 && c < 127 ? String.fromCharCode(c) : c === 10 ? '' : c === 13 ? '' : '';
      }).join('');
    }
  }

  const errTag = d.err
    ? h('span', { style: 'color:#ef4444;font-size:.75rem;margin-left:4px' }, '(' + d.err + ')')
    : '';

  const row = h('div', { style: 'display:flex;gap:8px;padding:1px 0;border-bottom:1px solid #1e293b' },
    h('span', { style: 'color:#64748b;min-width:64px' }, d._ts || ''),
    h('span', {
      style: 'color:' + (isRx ? '#22c55e' : '#f59e0b') + ';font-weight:600;min-width:20px'
    }, isRx ? 'RX' : 'TX'),
    h('span', { style: 'color:#38bdf8;min-width:28px' }, '[' + d.len + ']'),
    h('span', {}, content, errTag)
  );
  return row;
}

let _streamEl = null, _rendered = 0;

function renderData() {
  // Incremental: chỉ append rows mới thay vì rebuild toàn bộ
  if (_streamEl && S.page === 'uart') {
    while (_rendered < S.stream.length) {
      const row = fmtRow(S.stream[_rendered]);
      if (row) _streamEl.append(row);
      _rendered++;
    }
    _streamEl.scrollTop = _streamEl.scrollHeight;
    return _streamEl.parentElement;
  }

  _rendered = 0;
  _streamEl = h('div', { cls: 'stream', style: 'flex:1;overflow-y:auto;min-height:0' });
  S.stream.forEach((d, i) => {
    const r = fmtRow(d);
    if (r) _streamEl.append(r);
    _rendered = i + 1;
  });
  setTimeout(() => { if (_streamEl) _streamEl.scrollTop = _streamEl.scrollHeight }, 10);

  const txInput = h('input', {
    type: 'text',
    placeholder: 'Gửi serial...',
    style: 'flex:1;padding:2px 6px;background:transparent;color:#e2e8f0;border:none;outline:none;font-size:.8rem;font-family:monospace;min-width:0'
  });

  const sendTx = () => {
    const v = txInput.value.trim();
    if (!v) return;
    fetch('/api/uart/tx', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ data: v })
    });
    txInput.value = '';
  };

  txInput.addEventListener('keydown', e => { if (e.key === 'Enter') sendTx() });

  const toolbar = h('div', {
    style: 'display:flex;align-items:center;gap:40px;margin-bottom:8px'
  },
    h('h3', {
      style: 'margin:0;font-size:.9rem;color:#94a3b8;text-transform:uppercase;letter-spacing:.05em;white-space:nowrap'
    }, 'UART Real-time'),
    h('div', {
      style: 'display:flex;align-items:center;gap:6px;flex:1;min-width:0;background:#1e293b;border:1px solid #334155;border-radius:6px;padding:3px 4px'
    },
      txInput,
      h('button', {
        style: 'padding:3px 12px;background:#2563eb;color:white;border:none;border-radius:4px;cursor:pointer;font-size:.78rem;font-weight:600;white-space:nowrap',
        onclick: sendTx
      }, 'Gửi')
    ),
    h('label', { cls: 'chk', style: 'font-size:.78rem' },
      h('input', {
        type: 'checkbox',
        ...(S.hexView ? { checked: '' } : {}),
        onchange: e => { S.hexView = e.target.checked; _streamEl = null; _rendered = 0; render() }
      }),
      h('span', { cls: 'chk-box' }),
      h('span', { style: 'color:#94a3b8' }, 'HEX')
    ),
    h('button', {
      style: 'padding:3px 10px;background:#334155;color:#94a3b8;border:1px solid #475569;border-radius:4px;cursor:pointer;font-size:.75rem',
      onclick: () => { S.stream = []; _streamEl = null; _rendered = 0; render() }
    }, 'Xoá')
  );

  return h('div', {
    cls: 'card',
    style: 'flex:1;display:flex;flex-direction:column;padding-bottom:8px;min-height:0;overflow:hidden'
  }, toolbar, _streamEl);
}
