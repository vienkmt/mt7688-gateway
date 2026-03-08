// --- System page ---

async function loadSysVersion() {
  try {
    const r = await fetch('/api/version');
    if (r.ok) {
      S.sys.version = await r.json();
      render();
      return;
    }
  } catch (_) {}
  S.sys.version = { version: '-', build_date: '-', git_commit: '-' };
  render();
}

async function loadUpgradeUrl() {
  try {
    const r = await fetch('/api/upgrade/url');
    if (r.ok) {
      const d = await r.json();
      S.sys.upgradeUrl = d.url || '';
      render();
    }
  } catch (_) {}
}

async function saveUpgradeUrl(input) {
  const url = input.value.trim();
  try {
    const r = await fetch('/api/upgrade/url', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ url })
    });
    if (r.ok) {
      S.sys.upgradeUrl = url;
      toast('Đã lưu URL', 'ok');
    } else {
      toast('Lỗi lưu URL', 'err');
    }
  } catch (_) {
    toast('Lỗi kết nối', 'err');
  }
}

async function checkUpgrade() {
  S.sys.checkingUpdate = true;
  S.sys.updateInfo = null;
  render();
  try {
    const r = await fetch('/api/upgrade/check');
    if (r.ok) {
      S.sys.updateInfo = await r.json();
      S.sys.checkingUpdate = false;
      render();
    } else {
      S.sys.checkingUpdate = false;
      toast('Lỗi kiểm tra cập nhật', 'err');
      render();
    }
  } catch (_) {
    S.sys.checkingUpdate = false;
    toast('Lỗi kết nối', 'err');
    render();
  }
}

async function doRemoteUpgrade() {
  try {
    const r = await fetch('/api/upgrade/remote', { method: 'POST' });
    if (r.ok) toast('Đang cài đặt firmware mới...', 'ok');
    else toast('Lỗi cài đặt', 'err');
  } catch (_) {
    toast('Lỗi kết nối', 'err');
  }
}

async function doUploadIpk(fileInput) {
  const file = fileInput.files[0];
  if (!file) {
    toast('Chọn file .ipk', 'err');
    return;
  }
  try {
    const buf = await file.arrayBuffer();
    const r = await fetch('/api/upgrade', {
      method: 'POST',
      headers: { 'Content-Type': 'application/octet-stream' },
      body: buf
    });
    if (r.ok) toast('Đã nâng cấp thành công', 'ok');
    else toast('Nâng cấp thất bại', 'err');
  } catch (_) {
    toast('Lỗi kết nối', 'err');
  }
}

async function doRestore(fileInput) {
  const file = fileInput.files[0];
  if (!file) {
    toast('Chọn file backup', 'err');
    return;
  }
  try {
    const buf = await file.arrayBuffer();
    const r = await fetch('/api/restore', {
      method: 'POST',
      headers: { 'Content-Type': 'application/octet-stream' },
      body: buf
    });
    if (r.ok) toast('Đã khôi phục cấu hình', 'ok');
    else toast('Khôi phục thất bại', 'err');
  } catch (_) {
    toast('Lỗi kết nối', 'err');
  }
}

async function doRestart() {
  if (!confirm('Khởi động lại thiết bị?')) return;
  try {
    const r = await fetch('/api/restart', { method: 'POST' });
    if (r.ok) toast('Đang khởi động lại...', 'ok');
    else toast('Lỗi khởi động lại', 'err');
  } catch (_) {
    toast('Lỗi kết nối', 'err');
  }
}

async function doFactoryReset() {
  if (!confirm('Tất cả cấu hình sẽ được reset về mặc định?')) return;
  try {
    const r = await fetch('/api/factory-reset', { method: 'POST' });
    if (r.ok) toast('Đang khôi phục mặc định...', 'ok');
    else toast('Lỗi khôi phục mặc định', 'err');
  } catch (_) {
    toast('Lỗi kết nối', 'err');
  }
}

function renderSystem() {
  if (!S.sys.version) {
    loadSysVersion();
    loadUpgradeUrl();
    return h('div', { cls: 'card' }, 'Đang tải...');
  }
  const v = S.sys.version;
  const ui = S.sys.updateInfo;

  // Section 1: Firmware info
  const updateSection = [];
  if (S.sys.checkingUpdate) {
    updateSection.push(h('div', { cls: 'spinner' }));
  } else if (ui) {
    if (ui.has_update) {
      updateSection.push(
        h('div', {
          style: 'margin-top:10px;padding:10px;background:#0f172a;border:1px solid #334155;border-radius:6px'
        },
          h('div', { cls: 'cf' },
            lbl('Phiên bản mới'), h('span', {}, ui.version || '-'),
            lbl('Kích thước'), h('span', {}, ui.size || '-'),
            lbl('Thay đổi'),
            h('span', {
              style: 'grid-column:2/5;white-space:pre-wrap;font-size:.8rem;color:#94a3b8'
            }, ui.changelog || '-')
          ),
          h('button', {
            cls: 'save-btn',
            style: 'background:#16a34a',
            onclick: doRemoteUpgrade
          }, 'Tải & Cài đặt')
        )
      );
    } else {
      updateSection.push(
        h('div', {
          style: 'color:#22c55e;font-size:.85rem;margin-top:8px'
        }, 'Đã là phiên bản mới nhất')
      );
    }
  }

  const firmware = h('div', { cls: 'card' },
    h('h3', {}, 'Firmware'),
    h('div', {
      cls: 'fw-meta',
      style: 'display:flex;gap:20px;align-items:center;margin-bottom:8px'
    },
      h('span', { style: 'font-size:.85rem' },
        h('span', { style: 'color:#64748b' }, 'Phiên bản: '),
        h('span', { style: 'color:#e2e8f0' }, 'v' + (v.version || '-'))
      ),
      h('span', { style: 'font-size:.85rem' },
        h('span', { style: 'color:#64748b' }, 'Build: '),
        h('span', { style: 'color:#e2e8f0' }, v.build_date || '-')
      ),
      h('span', { style: 'font-size:.85rem' },
        h('span', { style: 'color:#64748b' }, 'Commit: '),
        h('span', { style: 'font-family:monospace;color:#e2e8f0' }, v.git_commit || '-')
      )
    ),
    (() => {
      const inp = h('input', {
        type: 'text',
        value: S.sys.upgradeUrl,
        placeholder: 'https://example.com/ugate/latest.json',
        style: 'flex:1;padding:8px 12px;background:#0f172a;border:1px solid #334155;border-radius:6px;color:#e2e8f0;font-size:.85rem'
      });
      return h('div', {
        cls: 'fw-url-row',
        style: 'display:flex;gap:8px;align-items:center;margin-top:10px'
      },
        h('span', {
          style: 'color:#64748b;font-size:.85rem;white-space:nowrap'
        }, 'URL cập nhật'),
        inp,
        h('div', { cls: 'fw-url-btns', style: 'display:flex;gap:6px' },
          h('button', {
            cls: 'save-btn',
            style: 'margin:0;min-width:80px;white-space:nowrap',
            onclick: () => saveUpgradeUrl(inp)
          }, 'Lưu'),
          h('button', {
            cls: 'save-btn',
            style: 'margin:0;min-width:140px;background:#334155;white-space:nowrap',
            onclick: checkUpgrade
          }, 'Kiểm tra cập nhật')
        )
      );
    })(),
    ...updateSection
  );

  // Section 2: Upload IPK
  const ipkFile = h('input', { type: 'file', accept: '.ipk', style: 'display:none' });
  const ipkLabel = h('span', {
    style: 'color:#94a3b8;font-size:.85rem;flex:1;overflow:hidden;text-overflow:ellipsis;white-space:nowrap'
  }, 'Chưa chọn file');
  const ipkChooseBtn = h('button', {
    cls: 'save-btn',
    style: 'margin:0;background:#334155;white-space:nowrap',
    onclick: () => ipkFile.click()
  }, 'Chọn file');
  ipkFile.onchange = () => {
    ipkLabel.textContent = ipkFile.files[0]?.name || 'Chưa chọn file';
  };

  const uploadIpk = h('div', { cls: 'card' },
    h('h3', {}, 'Nâng cấp IPK'),
    h('div', { cls: 'ipk-row', style: 'display:flex;gap:8px;align-items:center' },
      ipkFile, ipkChooseBtn, ipkLabel,
      h('button', {
        cls: 'save-btn',
        style: 'margin:0;white-space:nowrap',
        onclick: () => doUploadIpk(ipkFile)
      }, 'Nâng cấp')
    )
  );

  // Section 3: Backup / Restore
  const restoreFile = h('input', { type: 'file', accept: '.config', style: 'display:none' });
  const restoreLabel = h('span', {
    style: 'color:#94a3b8;font-size:.85rem;flex:1;overflow:hidden;text-overflow:ellipsis;white-space:nowrap'
  }, 'Chưa chọn file');
  const restoreChooseBtn = h('button', {
    cls: 'save-btn',
    style: 'margin:0;background:#334155;white-space:nowrap',
    onclick: () => restoreFile.click()
  }, 'Chọn file');
  restoreFile.onchange = () => {
    restoreLabel.textContent = restoreFile.files[0]?.name || 'Chưa chọn file';
  };

  const backupRestore = h('div', { cls: 'card' },
    h('h3', {}, 'Sao lưu / Khôi phục'),
    h('div', {
      cls: 'bkrs-grid',
      style: 'display:grid;grid-template-columns:1fr 1fr;gap:10px'
    },
      h('button', {
        cls: 'save-btn',
        style: 'margin:0;white-space:nowrap;justify-self:start',
        onclick: () => { window.location.href = '/api/backup' }
      }, 'Tải backup'),
      h('div', { cls: 'bkrs-row', style: 'display:flex;gap:8px;align-items:center' },
        restoreFile, restoreChooseBtn, restoreLabel,
        h('button', {
          cls: 'save-btn',
          style: 'margin:0;white-space:nowrap',
          onclick: () => doRestore(restoreFile)
        }, 'Khôi phục')
      )
    )
  );

  // Section 4: System actions
  const actions = h('div', { cls: 'card' },
    h('h3', {}, 'Hành động hệ thống'),
    h('div', {
      cls: 'act-btns',
      style: 'display:flex;gap:10px;flex-wrap:wrap;justify-content:center'
    },
      h('button', {
        cls: 'save-btn',
        style: 'margin:0;min-width:160px;background:#b45309',
        onclick: doRestart
      }, 'Khởi động lại'),
      h('button', {
        cls: 'save-btn',
        style: 'margin:0;min-width:160px;background:#991b1b',
        onclick: doFactoryReset
      }, 'Khôi phục mặc định')
    )
  );

  // Section 5: Change password
  const inputSt = 'flex:1;padding:8px 12px;background:#0f172a;border:1px solid #334155;border-radius:6px;color:#e2e8f0;font-size:.85rem';
  const oldPw = h('input', { type: 'password', placeholder: 'Mật khẩu hiện tại', style: inputSt });
  const newPw = h('input', { type: 'password', placeholder: 'Mật khẩu mới (tối thiểu 4 ký tự)', style: inputSt });
  const confirmPw = h('input', { type: 'password', placeholder: 'Nhập lại mật khẩu mới', style: inputSt });

  const pwCard = h('div', { cls: 'card' },
    h('h3', {}, 'Đổi mật khẩu'),
    h('div', { cls: 'pw-row', style: 'display:flex;gap:8px;align-items:center' },
      oldPw, newPw, confirmPw,
      h('button', {
        cls: 'save-btn',
        style: 'margin:0;white-space:nowrap',
        onclick: async () => {
          if (!newPw.value || newPw.value.length < 4) {
            toast('Mật khẩu tối thiểu 4 ký tự', 'err');
            return;
          }
          if (newPw.value !== confirmPw.value) {
            toast('Mật khẩu nhập lại không khớp', 'err');
            return;
          }
          try {
            const r = await fetch('/api/password', {
              method: 'POST',
              headers: { 'Content-Type': 'application/json' },
              body: JSON.stringify({ old_password: oldPw.value, new_password: newPw.value })
            });
            if (r.ok) {
              toast('Đã đổi mật khẩu', 'ok');
              oldPw.value = '';
              newPw.value = '';
              confirmPw.value = '';
            } else {
              const d = await r.json();
              toast(d.error || 'Lỗi', 'err');
            }
          } catch (_) {
            toast('Lỗi kết nối', 'err');
          }
        }
      }, 'Lưu')
    )
  );

  return h('div', {}, firmware, uploadIpk, backupRestore, pwCard, actions);
}
