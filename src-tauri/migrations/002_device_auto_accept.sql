-- Cihaz başına "güvenli cihaz" işareti (kullanıcı isteği).
--
-- Açıkken o cihazdan gelen dosyalar onay sorulmadan kabul edilir. Varsayılan
-- KAPALI: güven, kullanıcının cihaz cihaz vereceği bir karar olmalı, uygulama
-- kendiliğinden varsaymamalı.
ALTER TABLE trusted_devices ADD COLUMN auto_accept INTEGER NOT NULL DEFAULT 0;
