** HƯỚNG DẪN XÓA DRIVER MMC **

Mã nguồn trong Ubuntu trên Dropbox hiện ở chế độ một cổng mạng (chỉ cổng 0 hoạt động).
Để chuyển sang chế độ năm cổng mạng, bạn cần xóa driver mmc và biên dịch lại firmware.

Bước 1: Sao chép tệp DTS
- Sao chép file dts từ folder này để ghi đè: target/linux/ramips/dts/mt7628an_mediatek_linkit-smart-7688.dts

Bước 2: Cập nhật cấu hình
- Sao chép config_bak để ghi đè file .config