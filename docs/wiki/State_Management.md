# State_Management

**Özet:** Çalışan rclone süreçlerinin, uygulama durumunun, task repository'sinin, scheduler'ın ve task PID'lerinin Tauri'nin `State` yapısı ile yönetimi. SQLite bağlantısı ve mount listesi de state içinde tutulur.

**Kütüphaneler:** serde, tokio, rusqlite, uuid, chrono, cron

**Bağlantılar:** [[Tauri_Backend]], [[Process_Manager]], [[Rclone_Integration]], [[Architecture_Overview]]

---

## Tauri State Deseni

Tauri v2'de state, `tauri::Builder::default().manage()` ile kaydedilir ve `State<'_, T>` ile komutlarda erişilir.

## Gerçekleşen State Yapısı

Proje dosyası: `src-tauri/src/state.rs`

```rust
pub struct AppState {
    pub processes: Arc<Mutex<HashMap<Uuid, ProcessHandle>>>,
    pub rclone_path: Arc<Mutex<Option<PathBuf>>>,
    pub mounts: Arc<Mutex<HashMap<Uuid, MountInfo>>>,
    pub task_repo: Arc<tokio::sync::Mutex<TaskRepo>>,
    pub scheduler: Arc<tokio::sync::Mutex<Option<TaskScheduler>>>,
    pub task_pids: Arc<tokio::sync::Mutex<HashMap<String, u32>>>,
}
```

- **`processes`**: Çalışan rclone süreçleri (PID + Child handle) — `std::sync::Mutex`
- **`rclone_path`**: Keşfedilen binary yolu (setup'ta belirlenir) — `std::sync::Mutex`
- **`mounts`**: Aktif mount'lar (remote + mount_point + status) — `std::sync::Mutex`
- **`task_repo`**: Görev veritabanı repository'si — `tokio::sync::Mutex` (async CRUD için)
- **`scheduler`**: Cron-tabanlı görev zamanlayıcı (opsiyonel) — `tokio::sync::Mutex`
- **`task_pids`**: Scheduler tarafından çalıştırılan task process'lerinin PID haritası (task_id → PID) — `tokio::sync::Mutex`

## State'i Kaydetme

State, `lib.rs` içinde `setup()` callback'inde oluşturulur:

```rust
.setup(|app| {
    let task_repo = Arc::new(tokio::sync::Mutex::new(TaskRepo::new(conn)));
    let scheduler = TaskScheduler::new(task_repo.clone(), rclone_path_arc, app.handle().clone());
    let state = AppState::new(task_repo, Some(scheduler));
    app.manage(state);
    tray::build_tray(app.handle())?;
    // scheduler.start() async spawn
    Ok(())
})
```

## Thread Güvenliği

- `processes`, `rclone_path`, `mounts`: `std::sync::Mutex` — kısa kilitlenmeler için async maliyeti gerekmez
- `task_repo`, `scheduler`, `task_pids`: `tokio::sync::Mutex` — async context'te .await içeren işlemler için
- Tüm alanlar `Arc<T>` ile çoklu task/thread arasında paylaşılır
- task_cmds handler'ları DB lock'u bırakıp scheduler await çağırır — deadlock önlenir

## Frontend State

- Her panel kendi `useState`'ini yönetir
- `TransferPanel`: `progress`, `speed`, `eta`, `processId`, `history` state'leri
- `ConfigPanel`: `remotes`, `loading`, `error`
- `MountPanel`: `mounts`, `loading`, `error`
- Event listener'lar `useEffect` + `listen()` ile yönetilir
- Gelecekte Zustand'a geçilebilir (3+ panel ortak state paylaştığında)
