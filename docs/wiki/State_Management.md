# State_Management

**Özet:** Çalışan rclone süreçlerinin ve uygulama durumunun Tauri'nin `State` yapısı ile yönetimi. SQLite bağlantısı ve mount listesi de state içinde tutulur. Aktif ve çalışır durumdadır.

**Kütüphaneler:** serde, tokio, rusqlite, uuid, chrono

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
    pub db: Arc<Mutex<Connection>>,
    pub mounts: Arc<Mutex<HashMap<Uuid, MountInfo>>>,
}
```

- **`processes`**: Çalışan rclone süreçleri (PID + Child handle)
- **`rclone_path`**: Keşfedilen binary yolu (setup'ta belirlenir)
- **`db`**: SQLite bağlantısı (Rusqlite bundled)
- **`mounts`**: Aktif mount'lar (remote + mount_point + status)

## State'i Kaydetme

State, `lib.rs` içinde `setup()` callback'inde oluşturulur:

```rust
.setup(|app| {
    let conn = Connection::open(&db_path)?;
    db::migrations::create_tables(&conn)?;
    let state = AppState::new(conn);
    if let Some(ref path) = rclone_path {
        *state.rclone_path.lock().unwrap() = Some(path.clone());
    }
    app.manage(state);
    Ok(())
})
```

## Thread Güvenliği

- Tüm alanlar `Arc<Mutex<T>>` ile korunur
- `std::sync::Mutex` tercih edilir — kısa kilitlenmeler için async maliyeti gerekmez
- Her Tauri komutu kendi async context'inde state'e erişir

## Frontend State

- Her panel kendi `useState`'ini yönetir
- `TransferPanel`: `progress`, `speed`, `eta`, `processId`, `history` state'leri
- `ConfigPanel`: `remotes`, `loading`, `error`
- `MountPanel`: `mounts`, `loading`, `error`
- Event listener'lar `useEffect` + `listen()` ile yönetilir
- Gelecekte Zustand'a geçilebilir (3+ panel ortak state paylaştığında)
