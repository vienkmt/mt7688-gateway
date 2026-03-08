const S = {
  page: 'login',
  ws: null,
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
  }
};

function $(s, p) {
  return (p || document).querySelector(s);
}

function h(tag, attrs, ...ch) {
  const el = document.createElement(tag);
  if (attrs) {
    Object.entries(attrs).forEach(([k, v]) => {
      if (k.startsWith('on')) el.addEventListener(k.slice(2), v);
      else if (k === 'cls') el.className = v;
      else el.setAttribute(k, v);
    });
  }
  ch.flat().forEach(c => el.append(typeof c === 'string' ? c : c));
  return el;
}

function render() {
  const root = $('#app');
  root.innerHTML = '';
  if (S.page === 'login') {
    renderLogin(root);
    return;
  }
  const pg = S.page;
  root.append(
    h('header', {},
      h('h1', {}, 'uGate - From UART to the world'),
      h('span', {},
        h('span', { cls: 'dot ' + (S.connected ? 'on' : 'off') }),
        'WS'
      )
    ),
    renderNav(),
    pg === 'status' ? renderStatus()
      : pg === 'config' ? renderConfig()
      : pg === 'network' ? renderNetwork()
      : pg === 'routing' ? renderRouting()
      : pg === 'toolbox' ? renderToolbox()
      : pg === 'uart' ? renderUart()
      : pg === 'system' ? renderSystem()
      : h('div', {})
  );
}

function renderLogin(root) {
  const inp = h('input', {
    type: 'password',
    placeholder: 'Mật khẩu',
    value: S.password,
    oninput: e => S.password = e.target.value,
    onkeydown: e => { if (e.key === 'Enter') doLogin() }
  });
  root.append(
    h('div', { cls: 'login' },
      h('h1', { style: 'color:#38bdf8;font-size:1.5rem' }, 'uGate - From UART to the world'),
      inp,
      h('button', { onclick: doLogin }, 'Đăng nhập'),
      S.loginErr ? h('div', { cls: 'error' }, S.loginErr) : h('span', {})
    )
  );
  setTimeout(() => inp.focus(), 100);
}

async function doLogin() {
  const r = await fetch('/api/login', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ password: S.password })
  });
  if (r.ok) {
    S.page = 'status';
    S.loginErr = '';
    connectWS();
    render();
  } else {
    S.loginErr = 'Sai mật khẩu';
    render();
  }
}

function renderNav() {
  const tabs = [
    ['status', 'Status'],
    ['config', 'Channels'],
    ['uart', 'UART'],
    ['network', 'Network'],
    ['routing', 'Routing'],
    ['toolbox', 'Toolbox'],
    ['system', 'System']
  ];
  return h('nav', {},
    ...tabs.map(([id, label]) =>
      h('button', {
        cls: S.page === id ? 'active' : '',
        onclick: () => { S.page = id; render() }
      }, label)
    )
  );
}
