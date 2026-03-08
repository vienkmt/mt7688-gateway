// Modal loader — fetch HTML templates từ server, inject dynamic data
// Dùng chung store (Vue reactive) từ app JS (global scope)

function openModal(id, title, url, onLoaded) {
  var existing = document.getElementById(id);
  if (existing) existing.remove();
  var overlay = document.createElement('div');
  overlay.className = 'modal-overlay';
  overlay.id = id;
  overlay.onclick = function(e) { if (e.target === overlay) overlay.remove(); };
  var modal = document.createElement('div');
  modal.className = 'modal';
  modal.style = 'position:relative;max-width:680px;max-height:85vh;overflow-y:auto';
  var header = document.createElement('h3');
  header.textContent = title;
  var closeBtn = document.createElement('button');
  closeBtn.className = 'modal-close';
  closeBtn.textContent = '\u2715';
  closeBtn.onclick = function() { overlay.remove(); };
  var body = document.createElement('div');
  modal.append(header, closeBtn, body);
  overlay.append(modal);
  document.body.append(overlay);
  // Fetch HTML template
  fetch(url).then(function(r) { return r.text(); }).then(function(html) {
    body.innerHTML = html;
    if (onLoaded) onLoaded(body);
  });
}

// --- Data Wrap Help ---
function showDataWrapHelp() {
  openModal('wrap-modal', 'Data Wrap', '/modals/help-data-wrap-format', function(body) {
    var dn = store.config ? store.config.general.device_name : 'ugate';
    var ts = Math.floor(Date.now() / 1000);
    var sample = JSON.stringify({device_name: dn, timestamp: ts, data: 'send from mcu'}, null, 2);
    var el = body.querySelector('#wrap-mqtt-sample');
    if (el) el.textContent = sample;
    el = body.querySelector('#wrap-get-sample');
    if (el) el.textContent = '?device_name=' + dn + '&timestamp=' + ts + '&data=send+from+mcu';
  });
}
