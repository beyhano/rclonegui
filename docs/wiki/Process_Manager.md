# Process_Manager

**Özet:** Rclone süreçlerinin asenkron yaşam döngüsünü yöneten katman. `tokio::process::Command` ile süreçleri spawn eder, çıkış durumlarını izler ve temiz sonlandırma sağlar. Aktif ve çalışır durumdadır.

**Kütüphaneler:** tokio (process, io-util, sync), serde, uuid, chrono

**Bağlantılar:** [[Tauri_Backend]], [[Rclone_Integration]], [[State_Management]], [[Event_Stream]]

---

## Gerçekleşen Mimari

Proje dosyası: `src-tauri/src/rclone/process.rs`

```rust
pub struct ProcessManager {
    processes: Arc<Mutex<HashMap<Uuid, ProcessHandle>>>,
    rclone_path: Arc<Mutex<Option<PathBuf>>>,
}
```

ProcessHandle `state.rs` içinde tanımlıdır, ProcessManager `process.rs` içinde.

## Yaşam Döngüsü

```
Başlat → Spawn(rclone komutu) → PID kaydet [[State_Management]]
   │
   ├── stdout/stderr → Event_Stream pipeline'ı
   │
   ├── Kullanıcı "Durdur" → kill(child.id()) → cleanup
   │
   ├── Process exit → durumu güncelle → event emit
   │
   └── Uygulama kapanıyor → tüm child process'leri temizle
```

## Temel İşlemler

### Süreç Başlatma

```rust
pub async fn spawn(&self, args: &[String]) -> Result<Uuid> {
    let child = Command::new(rclone_path)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)  // ÖNEMLİ: Tauri kapandığında child'ı da öldür
        .spawn()?;
    
    // PID'i state'e kaydet
    // stdout/stderr reader task'larını başlat → [[Event_Stream]]
}
```

### Süreç Durdurma

- `child.start_kill()` ile gracefull shutdown dene
- Timeout sonrası `child.kill()` ile zorla sonlandır
- Zombie process'leri önlemek için wait() çağır

### Uygulama Kapanışı

Tauri'nin `on_window_event` veya `RunEvent::Exit` handler'ı içinde:
1. Tüm `ProcessHandle`'ları dolaş
2. Her birine start_kill() gönder
3. Kısa timeout bekle
4. Kalanları kill() ile temizle

## Platform Notları

- **Linux**: `SIGTERM` → `SIGKILL` sırası
- **Windows**: `taskkill /PID` veya `Child::kill()`
- Tokio'nun `kill_on_drop(true)` özelliği her iki platformda da çalışır
