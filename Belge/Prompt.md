Selam! Tauri (v2) ve Rust kullanarak hem Windows hem de Linux platformlarında çalışacak bir Rclone GUI uygulaması geliştirmek istiyorum. 

Senden bu projenin Rust (Backend) mimarisini kurmanı ve Tauri komutlarını (commands) yazmanı bekliyorum. Kodları üretirken sana verdiğim kurallara kesinlikle uymalısın.

Projenin Temel Gereksinimleri:
1. Rclone Entegrasyonu: Sistemde kurulu olan (veya kullanıcının seçtiği) rclone binary'sini asenkron (Tokio kullanarak) çağırabilmeli, `rclone config`, `rclone copy/sync` ve `rclone mount` komutlarını yönetebilmeli.
2. State Yönetimi: Çalışan rclone süreçlerini (PID'lerini) Tauri'nin `State` yapısında tutmalı, uygulama kapandığında veya kullanıcı "Durdur" dediğinde bu süreçleri temiz bir şekilde sonlandırabilmeli.
3. Event Akışı: rclone'dan gelen stdout/stderr çıktılarını (özellikle ilerleme yüzdesi ve hızı) regex ile ayrıştırıp Tauri event'leri (`emit`) ile frontend'e paslamalı.

Önemli Kısıtlamalar (Tekrar):
- Kodlarında kesinlikle tek bir 'unsafe' kelimesi bile geçmeyecek.
- Ekleyeceğin tüm Rust bağımlılıkları sadece cargo add  kurulabilir temiz paketler olacak. Cargo.toml a Elle hiçbirsey yazilmayacak
