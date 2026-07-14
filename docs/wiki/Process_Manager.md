# Process_Manager

**Özet:** Rclone süreçlerinin asenkron yaşam döngüsünü yöneten katman. `tokio::process::Command` ile süreçleri spawn eder, çıkış durumlarını izler ve temiz sonlandırma sağlar. Hem manuel transfer/mount süreçleri hem de scheduler task süreçleri için iki farklı kill mekanizması kullanılır.

**Kütüphaneler:** tokio (process, io-util, sync), serde, uuid, chrono

**Bağlantılar:** [[Tauri_Backend]], [[Rclone_Integration]], [[State_Management]], [[Event_Stream]]

---

## Gerçekleşen Mimari

Proje dosyası: `src-tauri/src/rclone/process.rs`

```rust
pub struct ProcessManager {
    processes: Arc<Mutex<HashMap<Uuid, ProcessHandle>>>,
}
```

## Yaşam Döngüsü

```
Başlat → Spawn(rclone komutu) → PID kaydet [[State_Management]]
   │
   ├── stdout/stderr → Event_Stream pipeline'ı
   │
   ├── Kullanıcı "Durdur" → ProcessManager::stop(id) → child.start_kill()
   │
   ├── Scheduler task stop → taskkill/kill -9 (PID tabanlı)
   │
   ├── Process exit → rclone:process-completed event emit
   │
   └── Uygulama kapanıyor
       ├── ProcessManager.cleanup_all() → state.processes.clear()
       ├── task_pids → taskkill/kill -9 ile tüm scheduler PID'leri öldür
       └── scheduler.stop() → cancel_tokens ile cron döngüleri durdur
```

## İki Farklı Kill Mekanizması

| Yöntem | Kullanım Yeri | Mekanizma |
|---|---|---|
| `ProcessManager::stop()` | `rclone_stop`, `rclone_unmount` | `ProcessHandle`'ı map'ten kaldır → `kill_on_drop(true)` ile child sonlanır |
| `task_stop` | `task_cmds.rs::task_stop` | `state.task_pids[id]`'den PID al → `taskkill /F` / `kill -9` |

## Temel İşlemler

### Süreç Başlatma (ProcessManager)

```rust
pub async fn spawn(&self, args: &[String]) -> Result<Uuid> {
    let child = Command::new(rclone_path)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)  // ÖNEMLİ: Tauri kapandığında child'ı da öldür
        .spawn()?;
}
```

### Scheduler Task Başlatma (engine.rs)

```rust
let mut child = tokio::process::Command::new(rclone_path)
    .args(&args)
    .kill_on_drop(true)
    .stdout(Stdio::piped())
    .stderr(Stdio::piped())
    .spawn()?;

// PID tracking — task_stop komutuyla PID üzerinden öldürme
let pid = child.id().unwrap_or(0);
state.task_pids.lock().await.insert(task.id.clone(), pid);
```

### Süreç Durdurma

- **ProcessManager.stop(uuid)**: `ProcessHandle`'ı map'ten kaldır → `kill_on_drop(true)` trigger
- **task_stop(id)**: `state.task_pids[id]` → PID bul → platform-agnostik kill
  - Windows: `taskkill /PID {pid} /F`
  - Unix: `kill -9 {pid}`

### PID Tabanlı Kill (task_stop / Exit cleanup)

```rust
#[cfg(windows)]
tokio::process::Command::new("taskkill")
    .args(&["/PID", &pid.to_string(), "/F"])
    .output().await?;

#[cfg(not(windows))]
tokio::process::Command::new("kill")
    .arg("-9")
    .arg(pid.to_string())
    .output().await?;
```

### Uygulama Kapanışı

Exit handler'da üç aşamalı cleanup:

```rust
// 1. ProcessManager — state.processes'teki tüm rclone child'ları temizle
let pm = ProcessManager::new(state.processes.clone());
let _ = pm.cleanup_all();

// 2. task_pids — scheduler task PID'lerini platform kill ile öldür
for pid in pids_to_kill {
    taskkill /PID {pid} /F  // veya kill -9
}
task_pids.lock().await.clear();

// 3. Scheduler — cron döngülerini durdur
if let Some(scheduler) = guard.take() {
    scheduler.stop().await;
}
```

## Platform Notları

- **Linux**: `SIGKILL` (`kill -9`)
- **Windows**: `taskkill /PID /F`
- Tokio'nun `kill_on_drop(true)` özelliği her iki platformda da çalışır
- PID tabanlı kill (`taskkill`/`kill -9`) scheduler task'leri için kullanılır — ProcessHandle'a erişim gerekmez
