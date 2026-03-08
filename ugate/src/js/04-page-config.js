// --- Config page ---

function renderConfig() {
  if (!S.config) {
    loadConfig();
    return h('div', { cls: 'card' }, 'Đang tải...');
  }
  const c = S.config;

  // General
  const general = h('div', { cls: 'card' },
    h('h3', { style: 'display:flex;align-items:center;gap:8px' },
      'General', helpLink('Help', () => showDataWrapHelp())
    ),
    h('div', { cls: 'cf cf-3' },
      lbl('Tên thiết bị'), inp('text', c.general, 'device_name'),
      lbl('Data Wrap'),
      h('label', { cls: 'chk' },
        h('input', {
          type: 'checkbox',
          ...(c.general.wrap_json ? { checked: '' } : {}),
          onchange: e => {
            c.general.wrap_json = e.target.checked;
            if (e.target.checked) showDataWrapHelp();
            render();
          }
        }),
        h('span', { cls: 'chk-box' }),
        h('span', { style: 'color:#e2e8f0;font-size:.85rem' },
          c.general.wrap_json ? 'Bật' : 'Tắt')
      ),
      lbl('Text encoding'),
      h('label', { cls: 'chk' },
        h('input', {
          type: 'checkbox',
          ...(c.general.data_as_text ? { checked: '' } : {}),
          onchange: e => { c.general.data_as_text = e.target.checked; render() }
        }),
        h('span', { cls: 'chk-box' }),
        h('span', { style: 'color:#e2e8f0;font-size:.85rem' },
          c.general.data_as_text ? 'Text' : 'Hex')
      )
    )
  );

  // MQTT
  const mqtt = h('div', { cls: 'card' },
    chHdr('MQTT', c.mqtt),
    ...(c.mqtt.enabled ? [
      h('div', { cls: 'cf cf-3' },
        lbl('Broker'), inp('text', c.mqtt, 'broker'),
        lbl('Port'), inp('number', c.mqtt, 'port'),
        lbl('TLS'),
        h('label', { cls: 'chk' },
          h('input', {
            type: 'checkbox',
            ...(c.mqtt.tls ? { checked: '' } : {}),
            onchange: e => { c.mqtt.tls = e.target.checked }
          }),
          h('span', { cls: 'chk-box' }),
          h('span', { style: 'color:#e2e8f0;font-size:.85rem' },
            c.mqtt.tls ? 'Bật' : 'Tắt')
        ),
        lbl('Client ID'),
        h('input', {
          type: 'text',
          value: S.status.mqtt ? S.status.mqtt.client_id || '' : '',
          readonly: '',
          cls: 'inp',
          style: 'opacity:.6;cursor:default',
          placeholder: 'Chưa kết nối',
          title: 'Tự sinh ngẫu nhiên mỗi lần kết nối'
        }),
        lbl('Username'), inp('text', c.mqtt, 'username'),
        lbl('Mật khẩu'), inp('password', c.mqtt, 'password'),
        lbl('QoS'), slct(c.mqtt, 'qos', [['0', '0'], ['1', '1'], ['2', '2']]),
        lbl('Pub Topic'), inp('text', c.mqtt, 'topic'),
        lbl('Sub Topic'), inp('text', c.mqtt, 'sub_topic')
      )
    ] : [])
  );

  // HTTP
  const http = h('div', { cls: 'card' },
    chHdr('HTTP', c.http),
    ...(c.http.enabled ? [
      h('div', { cls: 'cf', style: 'grid-template-columns:auto 1fr auto 2fr' },
        lbl('Phương thức'), slct(c.http, 'method', [['post', 'POST'], ['get', 'GET']]),
        lbl('URL'), inp('text', c.http, 'url')
      )
    ] : [])
  );

  // TCP
  const tcpFields = [
    lbl('Chế độ'),
    slct(c.tcp, 'mode', [
      ['server', 'Server (lắng nghe)'],
      ['client', 'Client (kết nối tới)']
    ], false, () => render())
  ];
  if (c.tcp.mode === 'server') {
    tcpFields.push(lbl('Cổng lắng nghe'), inp('number', c.tcp, 'server_port'));
  }
  if (c.tcp.mode === 'client') {
    tcpFields.push(
      lbl('Địa chỉ server'), inp('text', c.tcp, 'client_host'),
      lbl('Cổng đích'), inp('number', c.tcp, 'client_port')
    );
  }
  const tcp = h('div', { cls: 'card' },
    chHdr('TCP', c.tcp),
    ...(c.tcp.enabled ? [h('div', { cls: 'cf' }, ...tcpFields)] : [])
  );

  return h('div', {},
    general, mqtt, http, tcp,
    h('button', { cls: 'save-btn', onclick: saveConfig }, 'Lưu cấu hình')
  );
}

async function loadConfig() {
  const r = await fetch('/api/config');
  if (r.ok) {
    S.config = await r.json();
    render();
  }
}

async function saveConfig() {
  try {
    const r = await fetch('/api/config', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify(S.config)
    });
    if (r.ok) toast('Đã lưu cấu hình', 'ok');
    else toast('Lưu thất bại', 'err');
  } catch (e) {
    toast('Lỗi kết nối', 'err');
  }
}
