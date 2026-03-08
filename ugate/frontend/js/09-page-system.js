// --- System page Vue component ---

const SystemPage = {
  template: `
    <div v-if="!store.sys.version" class="card">Đang tải...</div>
    <div v-else>
      <div class="card">
        <h3>Firmware</h3>
        <div class="fw-meta" style="display:flex;gap:20px;align-items:center;margin-bottom:8px">
          <span style="font-size:.85rem">
            <span style="color:#64748b">Phiên bản: </span>
            <span style="color:#e2e8f0">v{{ v.version || '-' }}</span>
          </span>
          <span style="font-size:.85rem">
            <span style="color:#64748b">Build: </span>
            <span style="color:#e2e8f0">{{ v.build_date || '-' }}</span>
          </span>
          <span style="font-size:.85rem">
            <span style="color:#64748b">Commit: </span>
            <span style="font-family:monospace;color:#e2e8f0">{{ v.git_commit || '-' }}</span>
          </span>
        </div>
        <div class="fw-url-row" style="display:flex;gap:8px;align-items:center;margin-top:10px">
          <span style="color:#64748b;font-size:.85rem;white-space:nowrap">URL cập nhật</span>
          <input ref="urlInput" type="text" v-model="store.sys.upgradeUrl"
            placeholder="https://example.com/ugate/latest.json"
            style="flex:1;padding:8px 12px;background:#0f172a;border:1px solid #334155;border-radius:6px;color:#e2e8f0;font-size:.85rem">
          <div class="fw-url-btns" style="display:flex;gap:6px">
            <button class="save-btn" style="margin:0;min-width:80px;white-space:nowrap"
                    @click="saveUrl">Lưu</button>
            <button class="save-btn" style="margin:0;min-width:140px;background:#334155;white-space:nowrap"
                    @click="checkUpgrade">Kiểm tra cập nhật</button>
          </div>
        </div>
        <div v-if="store.sys.checkingUpdate" class="spinner"></div>
        <div v-else-if="ui && ui.has_update"
             style="margin-top:10px;padding:10px;background:#0f172a;border:1px solid #334155;border-radius:6px">
          <div class="cf">
            <span class="lbl">Phiên bản mới</span><span>{{ ui.version || '-' }}</span>
            <span class="lbl">Kích thước</span><span>{{ ui.size || '-' }}</span>
            <span class="lbl">Thay đổi</span>
            <span style="grid-column:2/5;white-space:pre-wrap;font-size:.8rem;color:#94a3b8">{{ ui.changelog || '-' }}</span>
          </div>
          <button class="save-btn" style="background:#16a34a" @click="doRemoteUpgrade">Tải & Cài đặt</button>
        </div>
        <div v-else-if="ui && !ui.has_update" style="color:#22c55e;font-size:.85rem;margin-top:8px">
          Đã là phiên bản mới nhất
        </div>
      </div>

      <div class="card">
        <h3>Nâng cấp IPK</h3>
        <div class="ipk-row" style="display:flex;gap:8px;align-items:center">
          <input ref="ipkFile" type="file" accept=".ipk" style="display:none" @change="ipkName = $refs.ipkFile.files[0]?.name || ''">
          <button class="save-btn" style="margin:0;background:#334155;white-space:nowrap"
                  @click="$refs.ipkFile.click()">Chọn file</button>
          <span style="color:#94a3b8;font-size:.85rem;flex:1;overflow:hidden;text-overflow:ellipsis;white-space:nowrap">
            {{ ipkName || 'Chưa chọn file' }}
          </span>
          <button class="save-btn" style="margin:0;white-space:nowrap" @click="uploadIpk">Nâng cấp</button>
        </div>
      </div>

      <div class="card">
        <h3>Sao lưu / Khôi phục</h3>
        <div class="bkrs-grid" style="display:grid;grid-template-columns:1fr 1fr;gap:10px">
          <button class="save-btn" style="margin:0;white-space:nowrap;justify-self:start"
                  @click="window.location.href='/api/backup'">Tải backup</button>
          <div class="bkrs-row" style="display:flex;gap:8px;align-items:center">
            <input ref="restoreFile" type="file" accept=".config" style="display:none"
                   @change="restoreName = $refs.restoreFile.files[0]?.name || ''">
            <button class="save-btn" style="margin:0;background:#334155;white-space:nowrap"
                    @click="$refs.restoreFile.click()">Chọn file</button>
            <span style="color:#94a3b8;font-size:.85rem;flex:1;overflow:hidden;text-overflow:ellipsis;white-space:nowrap">
              {{ restoreName || 'Chưa chọn file' }}
            </span>
            <button class="save-btn" style="margin:0;white-space:nowrap" @click="doRestore">Khôi phục</button>
          </div>
        </div>
      </div>

      <div class="card">
        <h3>Đổi mật khẩu</h3>
        <div class="pw-row" style="display:flex;gap:8px;align-items:center">
          <input ref="oldPw" type="password" placeholder="Mật khẩu hiện tại" :style="inputSt">
          <input ref="newPw" type="password" placeholder="Mật khẩu mới (tối thiểu 4 ký tự)" :style="inputSt">
          <input ref="confirmPw" type="password" placeholder="Nhập lại mật khẩu mới" :style="inputSt">
          <button class="save-btn" style="margin:0;white-space:nowrap" @click="changePw">Lưu</button>
        </div>
      </div>

      <div class="card">
        <h3>Hành động hệ thống</h3>
        <div class="act-btns" style="display:flex;gap:10px;flex-wrap:wrap;justify-content:center">
          <button class="save-btn" style="margin:0;min-width:160px;background:#b45309" @click="doRestart">Khởi động lại</button>
          <button class="save-btn" style="margin:0;min-width:160px;background:#991b1b" @click="doFactoryReset">Khôi phục mặc định</button>
        </div>
      </div>
    </div>
  `,
  setup() {
    const v = Vue.computed(() => store.sys.version || {});
    const ui = Vue.computed(() => store.sys.updateInfo);
    const ipkName = Vue.ref('');
    const restoreName = Vue.ref('');
    const inputSt = 'flex:1;padding:8px 12px;background:#0f172a;border:1px solid #334155;border-radius:6px;color:#e2e8f0;font-size:.85rem';

    Vue.onMounted(() => {
      if (!store.sys.version) { loadSysVersion(); loadUpgradeUrl(); }
    });

    return { store, v, ui, ipkName, restoreName, inputSt, window };
  },
  methods: {
    async saveUrl() {
      const url = store.sys.upgradeUrl.trim();
      try {
        const r = await fetch('/api/upgrade/url', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ url })
        });
        if (r.ok) toast('Đã lưu URL', 'ok');
        else toast('Lỗi lưu URL', 'err');
      } catch (_) { toast('Lỗi kết nối', 'err'); }
    },
    async checkUpgrade() {
      store.sys.checkingUpdate = true;
      store.sys.updateInfo = null;
      try {
        const r = await fetch('/api/upgrade/check');
        if (r.ok) store.sys.updateInfo = await r.json();
        else toast('Lỗi kiểm tra cập nhật', 'err');
      } catch (_) { toast('Lỗi kết nối', 'err'); }
      store.sys.checkingUpdate = false;
    },
    async doRemoteUpgrade() {
      try {
        const r = await fetch('/api/upgrade/remote', { method: 'POST' });
        if (r.ok) toast('Đang cài đặt firmware mới...', 'ok');
        else toast('Lỗi cài đặt', 'err');
      } catch (_) { toast('Lỗi kết nối', 'err'); }
    },
    async uploadIpk() {
      const file = this.$refs.ipkFile.files[0];
      if (!file) { toast('Chọn file .ipk', 'err'); return; }
      try {
        const buf = await file.arrayBuffer();
        const r = await fetch('/api/upgrade', {
          method: 'POST',
          headers: { 'Content-Type': 'application/octet-stream' },
          body: buf
        });
        if (r.ok) toast('Đã nâng cấp thành công', 'ok');
        else toast('Nâng cấp thất bại', 'err');
      } catch (_) { toast('Lỗi kết nối', 'err'); }
    },
    async doRestore() {
      const file = this.$refs.restoreFile.files[0];
      if (!file) { toast('Chọn file backup', 'err'); return; }
      try {
        const buf = await file.arrayBuffer();
        const r = await fetch('/api/restore', {
          method: 'POST',
          headers: { 'Content-Type': 'application/octet-stream' },
          body: buf
        });
        if (r.ok) toast('Đã khôi phục cấu hình', 'ok');
        else toast('Khôi phục thất bại', 'err');
      } catch (_) { toast('Lỗi kết nối', 'err'); }
    },
    async changePw() {
      const newVal = this.$refs.newPw.value;
      if (!newVal || newVal.length < 4) { toast('Mật khẩu tối thiểu 4 ký tự', 'err'); return; }
      if (newVal !== this.$refs.confirmPw.value) { toast('Mật khẩu nhập lại không khớp', 'err'); return; }
      try {
        const r = await fetch('/api/password', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ old_password: this.$refs.oldPw.value, new_password: newVal })
        });
        if (r.ok) {
          toast('Đã đổi mật khẩu', 'ok');
          this.$refs.oldPw.value = '';
          this.$refs.newPw.value = '';
          this.$refs.confirmPw.value = '';
        } else {
          const d = await r.json();
          toast(d.error || 'Lỗi', 'err');
        }
      } catch (_) { toast('Lỗi kết nối', 'err'); }
    },
    async doRestart() {
      if (!confirm('Khởi động lại thiết bị?')) return;
      try {
        const r = await fetch('/api/restart', { method: 'POST' });
        if (r.ok) toast('Đang khởi động lại...', 'ok');
        else toast('Lỗi khởi động lại', 'err');
      } catch (_) { toast('Lỗi kết nối', 'err'); }
    },
    async doFactoryReset() {
      if (!confirm('Tất cả cấu hình sẽ được reset về mặc định?')) return;
      try {
        const r = await fetch('/api/factory-reset', { method: 'POST' });
        if (r.ok) toast('Đang khôi phục mặc định...', 'ok');
        else toast('Lỗi khôi phục mặc định', 'err');
      } catch (_) { toast('Lỗi kết nối', 'err'); }
    }
  }
};

async function loadSysVersion() {
  try {
    const r = await fetch('/api/version');
    if (r.ok) { store.sys.version = await r.json(); return; }
  } catch (_) {}
  store.sys.version = { version: '-', build_date: '-', git_commit: '-' };
}

async function loadUpgradeUrl() {
  try {
    const r = await fetch('/api/upgrade/url');
    if (r.ok) { const d = await r.json(); store.sys.upgradeUrl = d.url || ''; }
  } catch (_) {}
}
