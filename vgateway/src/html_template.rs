use crate::system_info::SystemInfo;

/// Format uptime seconds to Vietnamese d/h/m/s format
fn format_uptime(secs: f64) -> String {
    let total = secs as u64;
    let d = total / 86400;
    let h = (total % 86400) / 3600;
    let m = (total % 3600) / 60;
    let s = total % 60;
    if d > 0 {
        format!("{}n {}g {}p {}s", d, h, m, s)
    } else if h > 0 {
        format!("{}g {}p {}s", h, m, s)
    } else {
        format!("{}p {}s", m, s)
    }
}

/// Render the system monitor HTML page with current system info
pub fn render_page(info: &SystemInfo) -> String {
    let ram_pct = if info.ram_total_mb > 0.0 {
        (info.ram_used_mb / info.ram_total_mb * 100.0) as u8
    } else { 0 };

    format!(
        r#"<!DOCTYPE html>
<html><head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>MT7688 Monitor</title>
<style>
* {{ box-sizing: border-box; }}
body {{
    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
    min-height: 100vh;
    display: flex;
    justify-content: center;
    align-items: center;
    margin: 0;
    padding: 20px;
    font-family: 'Segoe UI', system-ui, sans-serif;
}}
.card {{
    background: #fff;
    border-radius: 16px;
    padding: 28px;
    box-shadow: 0 20px 60px rgba(0,0,0,0.3);
    min-width: 420px;
    max-width: 480px;
}}
h1 {{ margin: 0 0 4px; font-size: 22px; color: #333; text-align: center; }}
.subtitle {{ text-align: center; color: #888; font-size: 12px; margin-bottom: 20px; }}
.section {{ margin-bottom: 16px; }}
.section-title {{ font-size: 11px; color: #667eea; font-weight: 600; text-transform: uppercase; letter-spacing: 1px; margin-bottom: 8px; border-bottom: 1px solid #eee; padding-bottom: 4px; }}
.row {{ display: flex; justify-content: space-between; align-items: center; padding: 6px 0; font-size: 13px; }}
.label {{ color: #666; }}
.value {{ font-weight: 500; color: #333; }}
#uptime, #localtime {{ color: #667eea; font-variant-numeric: tabular-nums; }}
.progress-wrap {{ flex: 1; margin-left: 12px; }}
.progress-info {{ display: flex; justify-content: space-between; font-size: 11px; color: #666; margin-bottom: 3px; }}
.progress-bar {{ height: 6px; background: #eee; border-radius: 3px; overflow: hidden; }}
.progress-fill {{ height: 100%; border-radius: 3px; transition: width 0.3s; }}
.fill-ram {{ background: linear-gradient(90deg, #667eea, #764ba2); }}
.fill-disk {{ background: linear-gradient(90deg, #f093fb, #f5576c); }}
.ram-grid {{ display: grid; grid-template-columns: 1fr 1fr; gap: 4px 16px; font-size: 12px; margin-top: 8px; }}
.ram-grid .label {{ color: #888; }}
.ram-grid .value {{ color: #333; font-weight: 500; }}
.btn {{
    display: block; text-align: center; margin-top: 20px; padding: 12px;
    background: linear-gradient(135deg, #667eea, #764ba2);
    color: #fff; text-decoration: none; border-radius: 8px; font-weight: 500;
}}
.btn:hover {{ opacity: 0.9; }}
</style>
</head>
<body>
<div class="card">
    <h1>RMS7688 SOM</h1>
    <div class="subtitle">MediaTek MT7688 • Kernel {kernel}</div>

    <div class="section">
        <div class="section-title">Hệ thống</div>
        <div class="row"><span class="label">Thời gian chạy</span><span class="value" id="uptime">{uptime}</span></div>
        <div class="row"><span class="label">Thời gian thiết bị</span><span class="value" id="localtime">{localtime}</span></div>
        <div class="row"><span class="label">Tiến trình</span><span class="value" id="procs">{procs}</span></div>
    </div>

    <div class="section">
        <div class="section-title">Bộ nhớ RAM</div>
        <div class="row">
            <span class="label">Sử dụng</span>
            <div class="progress-wrap">
                <div class="progress-info"><span id="ram-text">{ram_used:.1} / {ram_total:.1} MB</span><span id="ram-pct">{ram_pct}%</span></div>
                <div class="progress-bar"><div class="progress-fill fill-ram" id="ram-bar" style="width:{ram_pct}%"></div></div>
            </div>
        </div>
        <div class="ram-grid">
            <span class="label">Tổng:</span><span class="value" id="ram-total">{ram_total:.1} MB</span>
            <span class="label">Khả dụng:</span><span class="value" id="ram-avail">{ram_avail:.1} MB</span>
            <span class="label">Đã dùng:</span><span class="value" id="ram-used">{ram_used:.1} MB</span>
            <span class="label">Buffered:</span><span class="value" id="ram-buf">{ram_buf:.1} MB</span>
            <span class="label">Cached:</span><span class="value" id="ram-cache">{ram_cache:.1} MB</span>
        </div>
    </div>

    <div class="section">
        <div class="section-title">Flash</div>
        <div class="row">
            <span class="label">Sử dụng</span>
            <div class="progress-wrap">
                <div class="progress-info"><span id="disk-text">{disk_used} / {disk_total}</span><span id="disk-pct">{disk_pct}%</span></div>
                <div class="progress-bar"><div class="progress-fill fill-disk" id="disk-bar" style="width:{disk_pct}%"></div></div>
            </div>
        </div>
    </div>

    <div class="section">
        <div class="section-title">Mạng</div>
        <div class="row"><span class="label">Địa chỉ IP</span><span class="value" id="ip">{ip}</span></div>
        <div class="row"><span class="label">IP công cộng</span><span class="value" id="extip">{ext_ip}</span></div>
        <div class="row"><span class="label">Lưu lượng</span><span class="value" id="net">RX {net_rx} / TX {net_tx}</span></div>
    </div>

    <a href="/config" class="btn">Cấu hình</a>
</div>
<script>
var uptimeSecs = {uptime_secs};
function fmtUptime(s) {{
    var d = Math.floor(s/86400), h = Math.floor(s%86400/3600), m = Math.floor(s%3600/60), sec = s%60;
    return d > 0 ? d+'n '+h+'g '+m+'p '+sec+'s' : h > 0 ? h+'g '+m+'p '+sec+'s' : m+'p '+sec+'s';
}}
setInterval(function() {{ uptimeSecs++; document.getElementById('uptime').textContent = fmtUptime(uptimeSecs); }}, 1000);
setInterval(function() {{
    fetch('/').then(r => r.text()).then(html => {{
        var doc = new DOMParser().parseFromString(html, 'text/html');
        ['ip','extip','net','procs','localtime','ram-total','ram-avail','ram-used','ram-buf','ram-cache'].forEach(function(id) {{
            var el = doc.getElementById(id);
            if (el) document.getElementById(id).textContent = el.textContent;
        }});
        ['ram','disk'].forEach(function(id) {{
            var t = doc.getElementById(id+'-text'), p = doc.getElementById(id+'-pct'), b = doc.getElementById(id+'-bar');
            if (t) document.getElementById(id+'-text').textContent = t.textContent;
            if (p) {{ document.getElementById(id+'-pct').textContent = p.textContent; document.getElementById(id+'-bar').style.width = p.textContent; }}
        }});
    }});
}}, 5000);
</script>
</body></html>"#,
        uptime = format_uptime(info.uptime_secs),
        uptime_secs = info.uptime_secs as u64,
        kernel = info.kernel,
        localtime = info.local_time,
        ram_total = info.ram_total_mb,
        ram_avail = info.ram_available_mb,
        ram_used = info.ram_used_mb,
        ram_buf = info.ram_buffered_mb,
        ram_cache = info.ram_cached_mb,
        ram_pct = ram_pct,
        disk_used = info.disk_used,
        disk_total = info.disk_total,
        disk_pct = info.disk_percent,
        ip = info.ip_address,
        ext_ip = info.external_ip,
        net_rx = info.net_rx,
        net_tx = info.net_tx,
        procs = info.processes,
    )
}
