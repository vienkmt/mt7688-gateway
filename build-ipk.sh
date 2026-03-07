#!/bin/bash
#
# Đóng gói ugate thành file IPK chuẩn OpenWrt
# =============================================
#
# IPK là định dạng package của OpenWrt (tương tự .deb của Debian).
# File IPK thực chất là 1 file tar.gz chứa 3 thành phần:
#   - debian-binary:   Phiên bản format (luôn là "2.0")
#   - control.tar.gz:  Metadata + scripts (postinst, prerm, conffiles)
#   - data.tar.gz:     Các file thực tế sẽ được giải nén vào hệ thống
#
# Sau khi tạo IPK, cài lên thiết bị bằng: opkg install <file>.ipk
# opkg sẽ tự động chạy postinst (enable + start service)
# Khi gỡ bằng opkg remove, prerm sẽ stop + disable service
#
# Cách dùng:
#   ./build-ipk.sh              # Cross-compile + đóng gói IPK
#   ./build-ipk.sh --skip-build # Chỉ đóng gói từ binary đã build sẵn
#   ./build-ipk.sh --help       # Hiển thị hướng dẫn
#
set -e  # Dừng ngay nếu bất kỳ lệnh nào lỗi

# ============================================
# CẤU HÌNH PACKAGE
# ============================================
# Đổi version khi release mới (phải khớp với Cargo.toml)
PKG_NAME="ugate"
PKG_VERSION="0.1.0"
PKG_RELEASE="1"              # Số lần release cùng version (tăng khi fix packaging)
PKG_ARCH="mipsel_24kc"       # Kiến trúc CPU của MT7628 (MIPS little-endian, 24KEc core)

# ============================================
# ĐƯỜNG DẪN
# ============================================
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ROOT_DIR="$SCRIPT_DIR"
TARGET="mipsel-unknown-linux-musl"               # Rust cross-compile target cho OpenWrt
BINARY="$ROOT_DIR/target/$TARGET/release/$PKG_NAME"  # Binary sau khi build
BUILD_DIR="$ROOT_DIR/target/ipk-build"            # Thư mục tạm để tạo cấu trúc IPK
IPK_DIR="$BUILD_DIR/$PKG_NAME"                    # Cấu trúc file sẽ được đóng gói
OUTPUT_DIR="$ROOT_DIR/target/ipk"                 # Thư mục chứa file .ipk cuối cùng

# ============================================
# XỬ LÝ THAM SỐ DÒNG LỆNH
# ============================================
SKIP_BUILD=false
for arg in "$@"; do
    case $arg in
        --skip-build|-s) SKIP_BUILD=true ;;
        --help|-h) head -20 "$0" | tail -17; exit 0 ;;
    esac
done

# ============================================
# BƯỚC 1: CROSS-COMPILE BINARY
# ============================================
# Dùng "cross" (Docker-based) để build Rust cho MIPS target
# Cần Docker/OrbStack đang chạy vì cross tạo container để compile
if [ "$SKIP_BUILD" = false ]; then
    echo ">>> [1/9] Cross-compile $PKG_NAME cho $TARGET..."
    if ! docker info &>/dev/null; then
        echo "Lỗi: Docker/OrbStack chưa chạy! (cross cần Docker để build cho MIPS)"
        exit 1
    fi
    cross +nightly build --target "$TARGET" --release -p "$PKG_NAME"
    echo ">>> Build xong"
fi

# Kiểm tra binary tồn tại (có thể đã build trước đó với --skip-build)
if [ ! -f "$BINARY" ]; then
    echo "Lỗi: Không tìm thấy binary: $BINARY"
    echo "  -> Chạy lại không có --skip-build để build trước"
    exit 1
fi

# ============================================
# BƯỚC 2: TẠO CẤU TRÚC THƯ MỤC IPK
# ============================================
# Cấu trúc này mô phỏng hệ thống file của OpenWrt
# Khi opkg install, data.tar.gz sẽ được giải nén vào / (root)
#   - usr/bin/ugate        -> /usr/bin/ugate         (binary chính)
#   - etc/config/ugate     -> /etc/config/ugate      (UCI config)
#   - etc/init.d/ugate     -> /etc/init.d/ugate      (procd init script)
echo ">>> [2/9] Tạo cấu trúc thư mục IPK..."
rm -rf "$BUILD_DIR" "$OUTPUT_DIR"
mkdir -p "$IPK_DIR"/{CONTROL,usr/bin,etc/config,etc/init.d}
mkdir -p "$OUTPUT_DIR"

# Copy binary vào vị trí cài đặt
cp "$BINARY" "$IPK_DIR/usr/bin/"
chmod 755 "$IPK_DIR/usr/bin/$PKG_NAME"
echo "    Binary: $(du -h "$BINARY" | cut -f1)"

# ============================================
# BƯỚC 3: TẠO CONTROL FILE (metadata của package)
# ============================================
# opkg đọc file này để biết tên, version, kiến trúc, dependencies
# "Depends: libc" vì binary link với musl libc
echo ">>> [3/9] Tạo control file..."
cat > "$IPK_DIR/CONTROL/control" << EOF
Package: $PKG_NAME
Version: ${PKG_VERSION}-${PKG_RELEASE}
Architecture: $PKG_ARCH
Maintainer: vienkmt
Description: IoT Gateway - UART to MQTT/HTTP/TCP with WebSocket
Depends: libc
EOF

# ============================================
# BƯỚC 4: KHAI BÁO CONFFILES (file config được giữ lại khi upgrade)
# ============================================
# Khi chạy "opkg upgrade", opkg sẽ KHÔNG ghi đè các file listed ở đây
# Người dùng đã chỉnh sửa config -> config được bảo toàn
echo ">>> [4/9] Khai báo conffiles..."
cat > "$IPK_DIR/CONTROL/conffiles" << EOF
/etc/config/ugate
EOF

# ============================================
# BƯỚC 5: TẠO POSTINST (chạy SAU khi install xong)
# ============================================
# $IPKG_INSTROOT != "" nghĩa là đang install vào image (offline),
# không phải install trực tiếp trên thiết bị -> không start service
# Khi install trực tiếp: enable service (tự động start khi boot) + start ngay
echo ">>> [5/9] Tạo postinst script..."
cat > "$IPK_DIR/CONTROL/postinst" << 'EOF'
#!/bin/sh
[ -n "$IPKG_INSTROOT" ] && exit 0
/etc/init.d/ugate enable
/etc/init.d/ugate start
EOF
chmod 755 "$IPK_DIR/CONTROL/postinst"

# ============================================
# BƯỚC 6: TẠO PRERM (chạy TRƯỚC khi remove/upgrade)
# ============================================
# Dừng service trước khi xóa binary, tránh crash
echo ">>> [6/9] Tạo prerm script..."
cat > "$IPK_DIR/CONTROL/prerm" << 'EOF'
#!/bin/sh
[ -n "$IPKG_INSTROOT" ] && exit 0
/etc/init.d/ugate stop
/etc/init.d/ugate disable
EOF
chmod 755 "$IPK_DIR/CONTROL/prerm"

# ============================================
# BƯỚC 7: TẠO UCI CONFIG MẶC ĐỊNH
# ============================================
# UCI (Unified Configuration Interface) là hệ thống config của OpenWrt
# File này chứa giá trị mặc định, người dùng có thể chỉnh sửa qua:
#   - Sửa trực tiếp: vi /etc/config/ugate
#   - Dùng UCI CLI: uci set ugate.mqtt.enabled='1' && uci commit ugate
#   - Qua Web UI của ugate (port 8888)
# Khi upgrade, file này KHÔNG bị ghi đè (nhờ conffiles ở bước 4)
echo ">>> [7/9] Tạo UCI config mặc định..."
cat > "$IPK_DIR/etc/config/ugate" << EOF
config ugate 'main'
    option enabled '1'

config uart 'uart'
    option device '/dev/ttyS1'
    option baudrate '115200'

config mqtt 'mqtt'
    option enabled '0'
    option broker '127.0.0.1'
    option port '1883'
    option username ''
    option password ''
    option client_id 'ugate'
    option topic_publish 'ugate/data'
    option topic_subscribe 'ugate/cmd'
    option qos '0'

config http 'http'
    option enabled '0'
    option url 'http://example.com/api'
    option interval '60'

config tcp 'tcp'
    option enabled '0'
    option mode 'server'
    option server_port '9000'
    option client_host ''

config web 'web'
    option port '8888'
    option password ''

config gpio 'gpio'
    option out1 '0'
    option out2 '1'
    option out3 '2'
    option out4 '3'
    option led '4'
EOF

# ============================================
# BƯỚC 8: TẠO INIT SCRIPT (procd service manager)
# ============================================
# procd là init system của OpenWrt (tương tự systemd của Linux)
# - START=99: khởi động sau cùng (sau network, wifi, ...)
# - STOP=10:  dừng sớm khi shutdown
# - respawn:  tự động restart nếu process crash (mặc định 3 lần/5 phút)
# - service_triggers: khi UCI config thay đổi, procd tự gọi reload_service
# - stdout/stderr=1: redirect log vào syslog (đọc bằng: logread | grep ugate)
echo ">>> [8/9] Tạo procd init script..."
cat > "$IPK_DIR/etc/init.d/ugate" << 'EOF'
#!/bin/sh /etc/rc.common

START=99
STOP=10
USE_PROCD=1
PROG=/usr/bin/ugate

start_service() {
    # Đọc config từ UCI, chỉ start nếu enabled=1
    local enabled
    config_load ugate
    config_get enabled main enabled 0

    [ "$enabled" = "1" ] || return 0

    procd_open_instance
    procd_set_param command $PROG
    procd_set_param respawn       # Tự động restart khi crash
    procd_set_param stdout 1      # Log stdout vào syslog
    procd_set_param stderr 1      # Log stderr vào syslog
    procd_close_instance
}

service_triggers() {
    # Tự động reload khi UCI config /etc/config/ugate thay đổi
    procd_add_reload_trigger "ugate"
}

reload_service() {
    stop
    start
}
EOF
chmod 755 "$IPK_DIR/etc/init.d/ugate"

# ============================================
# BƯỚC 9: ĐÓNG GÓI THÀNH FILE IPK
# ============================================
# Cấu trúc IPK (thực chất là tar.gz):
#   debian-binary    -> Nội dung: "2.0" (phiên bản format, bắt buộc)
#   control.tar.gz   -> Chứa: control, conffiles, postinst, prerm
#   data.tar.gz      -> Chứa: usr/bin/ugate, etc/config/ugate, etc/init.d/ugate
# Thứ tự 3 file trong tar PHẢI đúng: debian-binary, control, data
echo ">>> [9/9] Đóng gói IPK..."

cd "$BUILD_DIR"
echo "2.0" > debian-binary

# Nén metadata + scripts
cd "$IPK_DIR/CONTROL"
tar czf "$BUILD_DIR/control.tar.gz" ./*

# Nén các file cài đặt (binary + config + init script)
cd "$IPK_DIR"
tar czf "$BUILD_DIR/data.tar.gz" ./usr ./etc

# Gộp 3 thành phần thành file .ipk
cd "$BUILD_DIR"
IPK_FILE="$OUTPUT_DIR/${PKG_NAME}_${PKG_VERSION}-${PKG_RELEASE}_${PKG_ARCH}.ipk"
tar czf "$IPK_FILE" ./debian-binary ./control.tar.gz ./data.tar.gz

# ============================================
# HOÀN TẤT
# ============================================
echo ""
echo "=== IPK đã tạo thành công ==="
echo "File: $IPK_FILE"
echo "Size: $(du -h "$IPK_FILE" | cut -f1)"
echo ""
echo "Cài đặt lên thiết bị:"
echo "  scp $IPK_FILE root@192.168.2.171:/tmp/"
echo "  ssh root@192.168.2.171 'opkg install /tmp/${PKG_NAME}_*.ipk'"
echo ""
echo "Các lệnh hữu ích sau khi cài:"
echo "  /etc/init.d/ugate status     # Kiểm tra trạng thái"
echo "  /etc/init.d/ugate restart    # Khởi động lại"
echo "  logread | grep ugate         # Xem log"
echo "  uci show ugate               # Xem config"
echo "  opkg remove ugate            # Gỡ cài đặt"
