// --- WebSocket ---

function connectWS() {
  if (S.ws) S.ws.close();
  const ws = new WebSocket('ws://' + location.host + '/ws');

  ws.onopen = () => {
    S.connected = true;
    render();
  };

  ws.onclose = () => {
    S.connected = false;
    render();
    setTimeout(connectWS, 3000);
  };

  ws.onmessage = e => {
    try {
      const d = JSON.parse(e.data);

      if (d.type === 'status') {
        S.status = d;
        if (S.page === 'status') updateStatus();
      } else if (d.type === 'toolbox') {
        if (S.toolbox) {
          if (d.done) {
            S.toolbox.running = false;
            S.toolbox.lines.push('--- done (exit code: ' + d.code + ') ---');
          } else if (d.line != null) {
            S.toolbox.lines.push(d.line);
          }
          if (S.page === 'toolbox') renderToolboxStream();
        }
      } else if (d.type === 'syslog') {
        if (!S.syslog) S.syslog = { lines: [], running: false, filter: '' };
        if (d.stopped) {
          S.syslog.running = false;
          if (S.page === 'toolbox') render();
        } else if (d.line != null) {
          S.syslog.lines.push({ text: d.line, level: d.level || 'info' });
          if (S.syslog.lines.length > 200) S.syslog.lines.shift();
          if (S.page === 'toolbox') renderSyslogStream();
        }
      } else {
        d._ts = new Date().toLocaleTimeString('vi', {
          hour12: false, hour: '2-digit', minute: '2-digit', second: '2-digit'
        });
        S.stream.push(d);
        if (S.stream.length > 200) {
          S.stream.shift();
          _rendered = Math.max(0, _rendered - 1);
          if (_streamEl && _streamEl.firstChild) _streamEl.removeChild(_streamEl.firstChild);
        }
        if (S.page === 'uart' && _streamEl) {
          const row = fmtRow(d);
          if (row) _streamEl.append(row);
          _rendered = S.stream.length;
          _streamEl.scrollTop = _streamEl.scrollHeight;
        }
      }
    } catch (_) {}
  };

  S.ws = ws;
}

function sendGpio(pin, state) {
  if (S.ws && S.ws.readyState === 1) {
    S.ws.send(JSON.stringify({ cmd: 'gpio', pin: '' + pin, state }));
  }
}

async function checkSession() {
  try {
    const r = await fetch('/api/status');
    if (r.ok) {
      S.page = 'status';
      connectWS();
    }
  } catch (_) {}
  render();
}

checkSession();
