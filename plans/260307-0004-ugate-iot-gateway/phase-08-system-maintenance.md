# Phase 8: System Maintenance

**Priority:** High
**Status:** pending
**Effort:** 2 days
**Depends on:** Phase 3 (Web Server)

## Context

Các tính năng quản trị hệ thống: backup/restore config, factory reset, restart, reload.

## API Endpoints

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/backup` | Download config file |
| POST | `/api/restore` | Upload & apply config |
| POST | `/api/factory-reset` | Reset to default |
| POST | `/api/restart` | Reboot device |
| POST | `/api/reload` | Hot reload config |
| GET | `/api/version` | Version + build info |
| POST | `/api/upgrade` | Upload IPK thủ công |
| GET | `/api/upgrade/check` | Check update từ remote URL |
| POST | `/api/upgrade/remote` | Download & install từ remote |

## Implementation

### 1. Backup Config

```rust
// GET /api/backup
// Response: application/octet-stream
pub fn handle_backup() -> Response {
    let config_path = "/etc/config/ugate";
    match std::fs::read(config_path) {
        Ok(data) => Response::from_data(data)
            .with_header(header("Content-Type", "application/octet-stream"))
            .with_header(header("Content-Disposition", "attachment; filename=ugate.config")),
        Err(_) => Response::from_string("Config not found").with_status_code(404),
    }
}
```

### 2. Restore Config

```rust
// POST /api/restore
// Body: multipart/form-data with config file
pub fn handle_restore(body: &[u8]) -> Response {
    // Parse multipart and extract file
    let config_data = parse_multipart_file(body)?;

    // Validate UCI format
    if !is_valid_uci(&config_data) {
        return error_response("Invalid config format");
    }

    // Backup current config
    std::fs::copy("/etc/config/ugate", "/tmp/ugate.backup")?;

    // Write new config
    std::fs::write("/etc/config/ugate", config_data)?;

    // Apply via UCI
    Command::new("sh")
        .args(["-c", "uci commit ugate"])
        .status()?;

    json_response(&StatusResponse {
        success: true,
        message: "Config restored. Reload to apply."
    })
}
```

### 3. Factory Reset

```rust
// POST /api/factory-reset
pub fn handle_factory_reset() -> Response {
    // Default UCI config
    let default_config = include_str!("../../default-ugate.config");

    // Write default config
    std::fs::write("/etc/config/ugate", default_config)?;

    // Commit
    Command::new("sh")
        .args(["-c", "uci commit ugate"])
        .status()?;

    json_response(&StatusResponse {
        success: true,
        message: "Factory reset done. Restart to apply."
    })
}
```

### 4. Restart Device

```rust
// POST /api/restart
pub fn handle_restart() -> Response {
    // Send response first, then reboot
    std::thread::spawn(|| {
        std::thread::sleep(std::time::Duration::from_secs(1));
        Command::new("reboot").status().ok();
    });

    json_response(&StatusResponse {
        success: true,
        message: "Restarting..."
    })
}
```

### 5. Reload Config (Hot Reload)

```rust
// POST /api/reload
// Body: { "modules": ["mqtt", "http", "tcp", "all"] }
pub fn handle_reload(body: &str, config_tx: &watch::Sender<()>) -> Response {
    let req: ReloadRequest = serde_json::from_str(body)?;

    // Re-read UCI config
    // This triggers config_notify channel
    let _ = config_tx.send(());

    json_response(&StatusResponse {
        success: true,
        message: format!("Reloading: {:?}", req.modules)
    })
}
```

### 6. Version Info

```rust
// GET /api/version
pub fn handle_version() -> Response {
    json_response(&VersionInfo {
        version: env!("CARGO_PKG_VERSION"),
        build_date: env!("BUILD_DATE"),      // Set in build.rs
        git_commit: env!("GIT_COMMIT"),      // Set in build.rs
    })
}
```

### 7. Upload IPK (Manual)

```rust
// POST /api/upgrade
// Body: multipart/form-data with .ipk file
pub fn handle_upgrade_upload(body: &[u8]) -> Response {
    let file_data = parse_multipart_file(body)?;

    // Validate IPK format
    if !is_valid_ipk(&file_data) {
        return error_response("Invalid IPK file");
    }

    std::fs::write("/tmp/ugate.ipk", &file_data)?;

    // Install async (response first, then install)
    std::thread::spawn(|| {
        std::thread::sleep(Duration::from_secs(1));
        Command::new("opkg")
            .args(["install", "/tmp/ugate.ipk", "--force-reinstall"])
            .status().ok();
    });

    json_response(&StatusResponse {
        success: true,
        message: "Installing... Device will restart."
    })
}
```

### 8. Check Update Online

UCI config:
```
config upgrade
    option url 'https://example.com/ugate/latest.json'
    option auto_check '0'
```

Remote manifest (latest.json):
```json
{
  "version": "1.0.1",
  "url": "https://example.com/ugate/ugate_1.0.1-1_mipsel_24kc.ipk",
  "checksum": "sha256:abc123...",
  "size": 850000,
  "changelog": "- Bug fixes\n- Performance improvements"
}
```

```rust
// GET /api/upgrade/check
pub fn handle_upgrade_check(state: &AppState) -> Response {
    let config = state.get();
    let url = &config.upgrade.url;

    if url.is_empty() {
        return error_response("Upgrade URL not configured");
    }

    // Fetch manifest
    let manifest: UpgradeManifest = ureq::get(url)
        .call()?
        .into_json()?;

    let current = env!("CARGO_PKG_VERSION");
    let has_update = version_compare(&manifest.version, current) > 0;

    json_response(&UpgradeCheckResponse {
        current_version: current,
        latest_version: manifest.version,
        has_update,
        changelog: manifest.changelog,
        size: manifest.size,
    })
}
```

### 9. Remote Upgrade (Download & Install)

```rust
// POST /api/upgrade/remote
pub fn handle_upgrade_remote(state: &AppState) -> Response {
    let config = state.get();
    let url = &config.upgrade.url;

    // Fetch manifest
    let manifest: UpgradeManifest = ureq::get(url).call()?.into_json()?;

    // Download IPK to /tmp
    let ipk_data = ureq::get(&manifest.url).call()?.into_reader();
    let mut file = std::fs::File::create("/tmp/ugate.ipk")?;
    std::io::copy(&mut ipk_data.into_reader(), &mut file)?;

    // Verify checksum
    let hash = sha256_file("/tmp/ugate.ipk");
    if hash != manifest.checksum {
        std::fs::remove_file("/tmp/ugate.ipk").ok();
        return error_response("Checksum mismatch");
    }

    // Install async
    std::thread::spawn(|| {
        std::thread::sleep(Duration::from_secs(1));
        Command::new("opkg")
            .args(["install", "/tmp/ugate.ipk", "--force-reinstall"])
            .status().ok();
    });

    json_response(&StatusResponse {
        success: true,
        message: format!("Upgrading to v{}...", manifest.version)
    })
}
```

### 10. Vue.js MaintenanceView

```vue
<template>
  <div class="maintenance">
    <!-- Version & Upgrade -->
    <section>
      <h3>Firmware</h3>
      <div>Current: v{{ version.version }} ({{ version.build_date }})</div>

      <button @click="checkUpdate">Check Update</button>
      <div v-if="updateInfo">
        <span v-if="updateInfo.has_update" class="success">
          New version: v{{ updateInfo.latest_version }}
        </span>
        <span v-else>Already latest</span>
        <pre v-if="updateInfo.changelog">{{ updateInfo.changelog }}</pre>
        <button v-if="updateInfo.has_update" @click="remoteUpgrade">
          Download & Install
        </button>
      </div>

      <h4>Manual Upload</h4>
      <input type="file" @change="selectIpk" accept=".ipk">
      <button @click="uploadIpk" :disabled="!selectedIpk">Upload IPK</button>
    </section>

    <!-- Backup/Restore -->
    <section>
      <h3>Configuration</h3>
      <button @click="backup">Download Backup</button>
      <input type="file" @change="selectConfig" accept=".config">
      <button @click="restore" :disabled="!selectedConfig">Restore</button>
    </section>

    <!-- System -->
    <section>
      <h3>System</h3>
      <button @click="reload">Reload Config</button>
      <button class="warning" @click="restart">Restart</button>
      <button class="danger" @click="factoryReset">Factory Reset</button>
    </section>
  </div>
</template>

<script setup lang="ts">
const selectedFile = ref<File | null>(null)

function backup() {
  window.location.href = '/api/backup'
}

async function restore() {
  if (!selectedFile.value) return
  const formData = new FormData()
  formData.append('file', selectedFile.value)
  await fetch('/api/restore', { method: 'POST', body: formData })
}

async function factoryReset() {
  if (!confirm('Reset all settings to default?')) return
  await fetch('/api/factory-reset', { method: 'POST' })
}

async function reload() {
  await fetch('/api/reload', {
    method: 'POST',
    body: JSON.stringify({ modules: ['all'] })
  })
}

async function restart() {
  if (!confirm('Restart device?')) return
  await fetch('/api/restart', { method: 'POST' })
}
</script>
```

## Files to Create/Modify

| File | Action |
|------|--------|
| ugate/src/web/maintenance.rs | Create |
| ugate/src/web/mod.rs | Modify |
| ugate/src/web/server.rs | Modify - add routes |
| ugate/src/default-ugate.config | Create - default UCI |
| ugate/frontend/src/views/MaintenanceView.vue | Create |

## Todo

- [ ] Create web/maintenance.rs
- [ ] Add backup handler
- [ ] Add restore handler (multipart parse)
- [ ] Add factory reset handler
- [ ] Add restart handler
- [ ] Add reload handler
- [ ] Add version handler
- [ ] Add upgrade upload handler
- [ ] Add upgrade check handler
- [ ] Add upgrade remote handler
- [ ] Create default-ugate.config
- [ ] Add upgrade URL to UCI config
- [ ] Wire routes in server.rs
- [ ] Create MaintenanceView.vue
- [ ] Test backup download
- [ ] Test restore upload
- [ ] Test factory reset
- [ ] Test restart
- [ ] Test hot reload
- [ ] Test upgrade check online
- [ ] Test manual IPK upload
- [ ] Test remote upgrade

## Success Criteria

- [ ] Backup downloads valid UCI file
- [ ] Restore applies uploaded config
- [ ] Factory reset restores defaults
- [ ] Restart reboots device
- [ ] Reload applies config without restart
- [ ] Version shows correct info
- [ ] Check update fetches from remote URL
- [ ] Manual IPK upload works
- [ ] Remote upgrade downloads, verifies, installs

## Security Notes

- Tất cả API yêu cầu auth
- Factory reset cần confirm
- Restart cần confirm
- Validate config format trước khi restore

## Next Phase

Phase Final: OpenWrt Packaging
