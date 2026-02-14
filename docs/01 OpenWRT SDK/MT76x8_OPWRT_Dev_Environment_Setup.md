# Hướng dẫn cài đặt môi trường phát triển MT76x8 OpenWrt

**Công ty:** Shenzhen Bojing Network Technology Co., Ltd.
**Website:** www.BOJINGnet.com

---

## 1. Giới thiệu môi trường phát triển

OpenWrt có thể được biên dịch trên hầu hết các hệ thống Linux, bao gồm Ubuntu, Redhat, Debian, Fedora, v.v. Hướng dẫn này sẽ giới thiệu cách thiết lập môi trường phát triển ảo trên Windows bằng VMware + Ubuntu.

---

## 2. Cài đặt phần mềm ảo hóa (VMware)

### (1) Tải xuống VMware
Tải từ: https://www.vmware.com/

### (2) Cài đặt VMware
Tham khảo hướng dẫn: https://jingyan.baidu.com/album/08b6a591e505cb14a809220c.html

---

## 3. Cài đặt Ubuntu trên máy ảo (khuyến nghị phiên bản 3)

### (1) Tải Ubuntu
Tải từ: http://www.ubuntu.org.cn/download

### (2) Cài đặt Ubuntu
Tham khảo hướng dẫn: https://jingyan.baidu.com/article/54b6b9c0ffd0142d583b471f.html

### (3) Import Ubuntu từ cloud drive
- **Tên đăng nhập:** lzh
- **Mật khẩu:** 123

---

## 4. Cấu hình môi trường biên dịch

> **Lưu ý:** Nếu sử dụng Ubuntu từ cloud drive, bạn không cần cấu hình này.

OpenWrt cần một số thư viện phụ thuộc để biên dịch, vì vậy cần cài đặt chúng trước.

### (1) Cài đặt SSH
```bash
sudo apt-get install openssh-client openssh-server
```
(Cho phép kết nối từ máy chủ Windows đến máy ảo)

### (2) Cài đặt SVN
```bash
sudo apt-get install subversion
```

### (3) Cài đặt Git
```bash
sudo apt-get install git-core
```

### (4) Cài đặt Samba
```bash
sudo apt-get install samba
```

**Cấu hình Samba:** Thêm các dòng sau vào cuối file `/etc/samba/smb.conf` (chế độ ẩn danh):
```
[new]
comment = Samba server's new
path = /new  # Thư mục chia sẻ trên Ubuntu
public = yes
writable = yes
create mask = 0766
```

Tham khảo thêm: https://blog.csdn.net/qq_38265137/article/details/83150450

### (5) Cài đặt các thư viện phụ thuộc của OpenWrt
```bash
git clone https://github.com/openwrt/openwrt.git
```

### (6) Cài đặt các công cụ biên dịch
```bash
sudo apt-get install gcc g++ binutils patch bzip2 flex bison make \
  autoconf gettext texinfo unzip sharutils ncurses-term zlib1g-dev \
  libncurses5-dev gawk openssl libssl-dev
```

---

## 5. Tải mã nguồn OpenWrt (LEDE)

### (1) Giới thiệu OpenWrt
Tham khảo: https://openwrt.org/zh/about

### (2) Tải mã nguồn LEDE
Tham khảo: https://openwrt.org/docs/guide-developer/source-code/start

**Lệnh tải:**
```bash
git clone https://git.openwrt.org/openwrt/openwrt.git
```

**Hoặc từ GitHub:**
```bash
git clone https://github.com/openwrt/openwrt.git
```

---

## 6. Biên dịch mã nguồn OpenWrt

### 1. Chuẩn bị thư mục làm việc
Vào thư mục OpenWrt, sao chép `feeds.conf.default` thành `feeds.conf`:
```bash
cp feeds.conf.default feeds.conf
```

### 2. Cập nhật và cài đặt feed
```bash
./scripts/feeds update -a
./scripts/feeds install -a
```

### 3. Cấu hình biên dịch cho MT7628/MT7688

#### Mở menu cấu hình:
```bash
make menuconfig
```

**Chọn:**
- **Target System:** Ramips
- **Subtarget:** mt76x8

Sau khi chọn xong, thoát và lưu cấu hình.

### 4. Biên dịch
```bash
make V=s
```

**Lưu ý quan trọng:** OpenWrt tải tất cả mã nguồn cần thiết khi biên dịch, vì vậy lần đầu tiên sẽ mất nhiều thời gian. Nếu tải gói nào đó bị lỗi, hãy tìm gói đó trên mạng, tải xuống và sao chép vào thư mục `dl`.

### 5. Đường dẫn firmware được tạo

```
ls bin/targets/ramips/mt76x8/openwrt-ramips-mt76x8-mediatek_linkit-smart-7688-squashfs-sysupgrade.bin
```

---

## 7. Tạo Toolchain

### (1) Cấu hình trong menuconfig
Trong `make menuconfig`, chọn `CONFIG_MAKE_TOOLCHAIN`.

### (2) Tìm Toolchain sau khi biên dịch
```bash
ls bin/targets/ramips/mt76x8/openwrt-ramips-mt76x8-mediatek_linkit-smart-7688-squashfs-sysupgrade.bin
```

---

## 8. Cài đặt Cross-Compile Toolchain

### Giải nén và cấu hình Toolchain

#### a) Giải nén toolchain
Sao chép toolchain đã tải hoặc biên dịch vào thư mục `/opt`:
```bash
sudo tar -xvf OpenWrt-Toolchain-ramips-mt76x8_*.tar.bz2 -C /opt
```

#### b) Cấu hình biến môi trường
Chỉnh sửa file `/etc/bash.bashrc`:
```bash
sudo vi /etc/bash.bashrc
```

Thêm dòng sau vào cuối file:
```bash
export PATH=/opt/OpenWrt-Toolchain-ramips-mt76x8_*/toolchain-mipsel_24kec+dsp_gcc-*/bin:$PATH
```

> **Lưu ý:** Biến `STAGING_DIR` không bắt buộc, nhưng thiếu nó có thể gây cảnh báo (không ảnh hưởng kết quả biên dịch).

Lưu tệp và chạy:
```bash
source /etc/bash.bashrc
```

#### c) Kiểm tra
```bash
mipsel-openwrt-linux-uclibc-gcc -v
```
Nếu hiển thị phiên bản, cài đặt thành công.

### Cài đặt ứng dụng

#### d) Biên dịch chương trình Hello World
```c
#include <stdio.h>
int main(int argc, char *argv[])
{
    printf("hello world\n");
    return 0;
}
```

Biên dịch (dùng `file` để xem thông tin file đầu ra):
```bash
mipsel-openwrt-linux-uclibc-gcc -Wall -o hello_world hello_world.c
```

#### e) Sao chép ứng dụng đến board

**Nếu máy ảo và board có kết nối mạng:**
```bash
scp hello_world root@192.168.8.1:/tmp/
```

**Nếu máy ảo không thể kết nối với board:**
1. Cài đặt `tftpd` trên máy chủ Windows
2. Dùng Samba để chia sẻ file đã biên dịch vào thư mục `tftpd`
3. Đăng nhập board qua serial, download file bằng:
```bash
tftp -gr hello_world 192.168.8.xxx
```

Hoặc dùng SCP trên Ubuntu:
```bash
scp hello_world root@192.168.8.1:/tmp/
```

#### f) Chạy chương trình
Đăng nhập board qua serial, vào thư mục chứa file:
```bash
chmod +x hello_world
./hello_world
```

---

## 9. Thông tin Ubuntu từ Cloud Drive

### 1. Import máy ảo
Vào **Virtual Machine (F) → Open → Chọn file đã tải về**

### 2. Khởi động máy ảo
- **Tên đăng nhập:** lzh
- **Mật khẩu:** 123

### 3. Thư mục OpenWrt đã cấu hình
Đường dẫn: `/work/bojingnet`

> **Lưu ý:** Thư mục mặc định khi đăng nhập là home directory, nhưng thư mục dự án nằm ở `/work/bojingnet`. Dùng lệnh `cd /work/bojingnet` để vào.

---

## Tài liệu tham khảo

- **OpenWrt Official:** https://openwrt.org
- **Bojing Network:** www.BOJINGnet.com
