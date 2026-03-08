// --- Shared Vue components ---

const SignalBars = {
  props: ['dbm'],
  template: `
    <span class="wifi-signal">
      <i v-for="(ht, i) in [6,10,14,18]" :key="i"
         :style="{ height: ht + 'px' }"
         :class="{ active: i < strength }"></i>
    </span>
  `,
  computed: {
    strength() { return signalStrength(this.dbm); }
  }
};

const IfaceBadge = {
  props: ['dev'],
  template: `
    <span :style="badgeStyle">{{ name }}</span>
  `,
  computed: {
    name() { return ifaceName(this.dev); },
    badgeStyle() {
      const c = ifaceBadgeColor(this.dev);
      return 'display:inline-flex;align-items:center;gap:4px;padding:2px 8px;background:'
        + c + '22;color:' + c + ';border-radius:4px;font-size:.75rem;font-weight:600';
    }
  }
};

const ChBadge = {
  props: ['state', 'enabled'],
  template: `<span :class="cls">{{ text }}</span>`,
  computed: {
    cls() {
      if (this.enabled === false) return 'ch-badge disabled';
      if (this.state === 'connected') return 'ch-badge connected';
      if (this.state === 'disabled') return 'ch-badge disabled';
      return 'ch-badge disconnected';
    },
    text() {
      if (this.enabled === false) return 'off';
      return this.state || 'disconnected';
    }
  }
};

const ProgressBar = {
  props: ['label', 'val', 'max', 'color', 'text'],
  template: `
    <div class="pbar-row">
      <label>{{ label }}</label>
      <div class="pbar">
        <div class="fill" :style="fillStyle"></div>
      </div>
      <span>{{ text }}</span>
    </div>
  `,
  computed: {
    pct() { return this.max > 0 ? Math.min(100, this.val / this.max * 100) : 0; },
    fillStyle() { return { width: this.pct + '%', background: this.color }; }
  }
};

const SwitchToggle = {
  props: ['modelValue'],
  emits: ['update:modelValue'],
  template: `
    <label class="sw">
      <input type="checkbox" :checked="modelValue"
             @change="$emit('update:modelValue', $event.target.checked)">
      <span class="sl"></span>
    </label>
  `
};

const ChannelHeader = {
  props: ['title', 'modelValue'],
  emits: ['update:modelValue'],
  template: `
    <h3>{{ title }}
      <switch-toggle :model-value="modelValue"
                     @update:model-value="$emit('update:modelValue', $event)"/>
    </h3>
  `
};

const PendingBanner = {
  template: `
    <div v-if="store.pendingChanges" class="pending-banner"
         style="background:#92400e;border:1px solid #f59e0b;border-radius:8px;padding:12px 16px;margin-bottom:12px;display:flex;justify-content:space-between;align-items:center">
      <span style="color:#fbbf24;font-weight:600;font-size:.85rem">
        ⚠ Có thay đổi chưa lưu vào flash
      </span>
      <div style="display:flex;gap:8px">
        <button style="padding:6px 16px;background:#334155;color:#94a3b8;border:1px solid #475569;border-radius:6px;cursor:pointer;font-size:.8rem"
                @click="revertChanges">Huỷ nháp</button>
        <button style="padding:6px 16px;background:#16a34a;color:white;border:none;border-radius:6px;cursor:pointer;font-weight:700;font-size:.8rem"
                @click="applyChanges">Áp dụng & Lưu flash</button>
      </div>
    </div>
  `,
  setup() { return { store }; },
  methods: {
    async applyChanges() {
      try {
        const r = await fetch('/api/network/apply', { method: 'POST' });
        if (r.ok) {
          toast('Đã lưu vào flash, đang khởi động lại...', 'ok');
          store.pendingChanges = false;
        } else toast('Lỗi áp dụng', 'err');
      } catch (_) { toast('Lỗi kết nối', 'err'); }
    },
    async revertChanges() {
      try {
        const r = await fetch('/api/network/revert', { method: 'POST' });
        if (r.ok) {
          toast('Đã huỷ thay đổi', 'ok');
          store.pendingChanges = false;
          store.net = null;
          store.ntp = null;
        } else toast('Lỗi huỷ', 'err');
      } catch (_) { toast('Lỗi kết nối', 'err'); }
    }
  }
};

const AppHeader = {
  template: `
    <header>
      <h1>uGate - From UART to the world</h1>
      <span>
        <span :class="'dot ' + (store.connected ? 'on' : 'off')"></span>
        WS
      </span>
    </header>
  `,
  setup() { return { store }; }
};

const NavBar = {
  template: `
    <nav>
      <button v-for="[id, label] in tabs" :key="id"
              :class="{ active: store.page === id }"
              @click="store.page = id">{{ label }}</button>
    </nav>
  `,
  setup() {
    const tabs = [
      ['status', 'Status'], ['config', 'Channels'], ['uart', 'UART'],
      ['network', 'Network'], ['routing', 'Routing'],
      ['toolbox', 'Toolbox'], ['system', 'System']
    ];
    return { store, tabs };
  }
};
