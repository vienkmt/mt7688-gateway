#!/bin/bash
#
# Deploy Script cho MT7628DAN (OpenWrt)
# =====================================
#
# Cách dùng:
#   ./deploy.sh              # Build và deploy ugate (mặc định)
#   ./deploy.sh ugate        # Build và deploy ugate
#   ./deploy.sh vgateway     # Build và deploy vgateway
#   ./deploy.sh --build-only # Chỉ build, không deploy
#   ./deploy.sh --skip-build # Chỉ deploy, không build lại
#
# Yêu cầu:
#   - Docker/OrbStack đang chạy
#   - cross tool đã cài (cargo install cross)
#   - Thiết bị OpenWrt kết nối được qua SSH
#

set -e  # Dừng ngay nếu có lỗi

# ============================================
# CẤU HÌNH - Thay đổi theo thiết bị của bạn
# ============================================

HOST="root@192.168.2.168"       # Địa chỉ SSH thiết bị
TARGET="mipsel-unknown-linux-musl"
REMOTE_DIR="/usr/bin"           # Thư mục cài đặt trên thiết bị

# ============================================
# XỬ LÝ THAM SỐ
# ============================================

APP="ugate"                     # App mặc định
SKIP_BUILD=false
BUILD_ONLY=false

for arg in "$@"; do
    case $arg in
        ugate|vgateway)
            APP="$arg"
            ;;
        --build-only|-b)
            BUILD_ONLY=true
            ;;
        --skip-build|-s)
            SKIP_BUILD=true
            ;;
        --help|-h)
            head -20 "$0" | tail -16
            exit 0
            ;;
    esac
done

BINARY="target/$TARGET/release/$APP"
REMOTE_PATH="$REMOTE_DIR/$APP"

# ============================================
# HIỂN THỊ THÔNG TIN
# ============================================

echo "┌─────────────────────────────────────┐"
echo "│  Deploy $APP → $HOST"
echo "└─────────────────────────────────────┘"

# ============================================
# BƯỚC 1: BUILD (nếu không skip)
# ============================================

if [ "$SKIP_BUILD" = false ]; then
    echo ""
    echo "▶ [1/4] Đang build $APP..."

    # Kiểm tra Docker/OrbStack
    if ! docker info &>/dev/null; then
        echo "❌ Docker/OrbStack chưa chạy!"
        echo "   Chạy: open -a OrbStack"
        exit 1
    fi

    # Build với cross
    cross +nightly build --target "$TARGET" --release -p "$APP"

    echo "✓ Build thành công"
else
    echo ""
    echo "▶ [1/4] Bỏ qua build (--skip-build)"
fi

# ============================================
# BƯỚC 2: KIỂM TRA BINARY
# ============================================

echo ""
echo "▶ [2/4] Kiểm tra binary..."

if [ ! -f "$BINARY" ]; then
    echo "❌ Không tìm thấy: $BINARY"
    echo "   Chạy lại không có --skip-build"
    exit 1
fi

# Hiển thị thông tin binary
SIZE=$(ls -lh "$BINARY" | awk '{print $5}')
echo "✓ $BINARY ($SIZE)"

# Nếu chỉ build, dừng ở đây
if [ "$BUILD_ONLY" = true ]; then
    echo ""
    echo "┌─────────────────────────────────────┐"
    echo "│  ✓ Build hoàn tất (--build-only)"
    echo "└─────────────────────────────────────┘"
    exit 0
fi

# ============================================
# BƯỚC 3: DỪNG PROCESS CŨ TRÊN THIẾT BỊ
# ============================================

echo ""
echo "▶ [3/4] Dừng process cũ trên thiết bị..."

ssh "$HOST" "killall $APP 2>/dev/null || true; sleep 1"
echo "✓ Đã dừng"

# ============================================
# BƯỚC 4: UPLOAD VÀ KHỞI ĐỘNG
# ============================================

echo ""
echo "▶ [4/4] Upload và khởi động..."

# Upload binary
scp -O "$BINARY" "$HOST:$REMOTE_PATH"
echo "✓ Upload thành công"

# Cấp quyền thực thi và chạy
ssh "$HOST" "chmod +x $REMOTE_PATH"

# Khởi động lại service (nếu có init script)
INIT_SCRIPT="/etc/init.d/$APP"
if ssh "$HOST" "[ -f $INIT_SCRIPT ]"; then
    ssh "$HOST" "$INIT_SCRIPT restart"
    echo "✓ Đã restart service"
else
    # Chạy trực tiếp nếu không có init script
    ssh "$HOST" "nohup $REMOTE_PATH > /var/log/$APP.log 2>&1 &"
    echo "✓ Đã chạy daemon"
fi

# ============================================
# KIỂM TRA KẾT QUẢ
# ============================================

echo ""
sleep 2

if ssh "$HOST" "pgrep -x $APP" &>/dev/null; then
    echo "┌─────────────────────────────────────┐"
    echo "│  ✓ $APP đang chạy!"
    echo "└─────────────────────────────────────┘"
    echo ""
    echo "Xem log: ssh $HOST 'logread | grep $APP'"
else
    echo "┌─────────────────────────────────────┐"
    echo "│  ❌ $APP không chạy được"
    echo "└─────────────────────────────────────┘"
    echo ""
    echo "Log gần nhất:"
    ssh "$HOST" "logread | tail -10"
    exit 1
fi
