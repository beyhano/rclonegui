# Rclone_Integration

**Özet:** Sistemde kurulu rclone binary'sini keşfeden, versiyonunu doğrulayan ve `config`, `copy/sync`, `mount`, `config create` komutlarını yöneten katman. Task scheduler engine'i de rclone binary'sini kullanarak cron görevlerini yürütür.

**Kütüphaneler:** tokio (process, io-util, sync), regex, serde, serde_json, uuid, chrono, rusqlite, cron

**Bağlantılar:** [[Tauri_Backend]], [[Process_Manager]], [[Event_Stream]], [[Architecture_Overview]], [[State_Management]]

---

## Modül Yapısı

```
src-tauri/src/rclone/
├── mod.rs           ← pub mod discovery, config, events, process
├── discovery.rs     ← Platform algılama, binary keşfi, executable doğrulama
├── process.rs       ← ProcessManager (spawn, stop, cleanup_all)
├── events.rs        ← Progress regex, line parse, event emit
└── config.rs        ← rclone config dump, Remote listesi

src-tauri/src/commands/
├── rclone_cmds.rs   ← 8 adet #[tauri::command]
└── task_cmds.rs     ← 8 adet #[tauri::command]

src-tauri/src/scheduler/
├── engine.rs        ← execute_task() — rclone spawn + progress/yakalama
└── ...
```

## Rclone Binary Yapısı

Binary'ler `rclone-bin/{platform}/` dizininde beklenir:

| Platform | Dizin | Binary Adı |
|---|---|---|
| Linux amd64 | `rclone-bin/linux/` | `rclone` |
| Linux arm64 | `rclone-bin/linux/` | `rclone` |
| Windows amd64 | `rclone-bin/windows/` | `rclone.exe` |
| macOS amd64 | `rclone-bin/osx-amd64/` | `rclone` |
| macOS arm64 | `rclone-bin/osx-arm64/` | `rclone` |

### Binary Bulma Stratejisi (`discovery::find_binary()`)

`setup()` anında şu sırayla aranır:
1. **`resource_dir()`** — production bundle (`tauri.conf.json` resources ile paketlenir)
2. **`CARGO_MANIFEST_DIR/..`** — `cargo test` / Cargo build zamanı proje kökü
3. **CWD** (current working directory) — `tauri dev` çalıştırılan dizin
4. **Exe ancestors** — binary yolundan yukarı çıkıp `rclone-bin/` klasörünü ara

Bulunamazsa `rclone_path` `None` kalır ve komutlar `"No rclone binary configured"` hatası döner.

## Tauri Komutları (8 adet — rclone_cmds.rs)

Tüm komutlar `commands/rclone_cmds.rs` içinde tanımlıdır ve `lib.rs`'de `invoke_handler` ile kaydedilmiştir.

### 1. `rclone_version`
```rust
#[tauri::command]
pub async fn rclone_version(state: State<'_, AppState>) -> Result<String, String>
```
- **İşlev**: `rclone version` çalıştırır, çıktıyı string olarak döner
- **Dönüş**: `"rclone v1.65.0\n..."` (ham stdout)

### 2. `rclone_config_list`
```rust
#[tauri::command]
pub async fn rclone_config_list(state: State<'_, AppState>) -> Result<Vec<Remote>, String>
```
- **İşlev**: `rclone config dump` JSON çıktısını parse eder
- **Dönüş**: `Vec<Remote>` — her remote için `{ name, type }`
- **Güvenlik**: Hassas alanlar (token, secret) otomatik atılır

### 3. `rclone_config_create` (YENİ)
```rust
#[tauri::command]
pub async fn rclone_config_create(
    state: State<'_, AppState>,
    name: String,
    provider: String,
    config: String, // JSON string of key-value pairs
) -> Result<(), String>
```
- **İşlev**: `rclone config create <name> <provider> --non-interactive <key> <value> ...`
- **Dönüş**: `()` — başarılı/başarısız durumu
- **Not**: Config JSON'dan key-value çiftleri parse edilip argüman olarak eklenir

### 4. `rclone_exec`
```rust
#[tauri::command]
pub async fn rclone_exec(
    app: AppHandle,
    state: State<'_, AppState>,
    args: Vec<String>,
) -> Result<String, String>
```
- **İşlev**: Rastgele rclone argümanları ile süreç başlatır
- **Dönüş**: UUID string
- **Yan etki**: `rclone:process-started` emit, `start_event_stream()` ile background task, `rclone:process-completed` emit

### 5. `rclone_stop`
```rust
#[tauri::command]
pub async fn rclone_stop(state: State<'_, AppState>, process_id: String) -> Result<(), String>
```

### 6. `rclone_mount`
```rust
#[tauri::command]
pub async fn rclone_mount(
    app: AppHandle,
    state: State<'_, AppState>,
    remote: String,
    mount_point: String,
) -> Result<String, String>
```

### 7. `rclone_unmount`
```rust
#[tauri::command]
pub async fn rclone_unmount(state: State<'_, AppState>, mount_id: String) -> Result<(), String>
```

### 8. `rclone_mount_list`
```rust
#[tauri::command]
pub fn rclone_mount_list(state: State<'_, AppState>) -> Result<Vec<MountInfo>, String>
```

## Scheduler Engine ile Rclone Etkileşimi (engine.rs)

`execute_task()` fonksiyonu rclone'u şu argümanlarla spawn eder:

```rust
let mut args = vec![task.operation.clone()];   // "copy", "sync", "move", "bisync"
args.push(task.source_provider.clone());         // "gdrive:/backups"
args.push(task.dest_provider.clone());           // "local:/mnt/disk"
for pattern in &task.exclude_patterns {
    args.push("--exclude".to_string());
    args.push(pattern.clone());                  // "*.tmp", "*.log"
}
args.push("--progress".to_string());             // rclone --progress
```

### Engine Yaşam Döngüsü

1. `tokio::process::Command::new(rclone_path).args(&args).spawn()`
2. PID alınır → `state.task_pids[task.id] = pid` (task_stop için)
3. stdout → `BufReader` → `parse_progress_line()` → `rclone:progress` emit
4. stderr → `BufReader` → `rclone:log` emit
5. `child.wait()` → exit code
6. PID temizlenir: `state.task_pids.remove(&task.id)`
7. `TaskResult` döner (success/error, timing)

## Config Yönetimi

`rclone/config.rs` içinde `config_list()` fonksiyonu:

```rust
pub async fn config_list(rclone_path: &Path) -> Result<Vec<Remote>, String> {
    let output = tokio::process::Command::new(rclone_path)
        .args(["config", "dump"])
        .output().await?;

    // HashMap<String, HashMap<String, Value>> parse
    // Her remote için name + type çıkar, hassas alanlar atılır
    // Alfabetik sıralanır
}
```

### Remote Yapısı

```rust
#[derive(Debug, Clone, Serialize)]
pub struct Remote {
    pub name: String,
    #[serde(rename = "type")]
    pub remote_type: String,   // "drive", "s3", "dropbox", etc.
}
```

## Event Türleri (9 adet)

Tüm event'ler `rclone:` namespace'i altında emit edilir.

| Event | Payload | Kaynak | Tetikleyici |
|---|---|---|---|
| `rclone:progress` | `ProgressPayload` | `events.rs` / `engine.rs` | Progress satırı eşleştiğinde |
| `rclone:log` | `{process_id, line}` | `events.rs` / `engine.rs` | Her stdout/stderr satırı |
| `rclone:process-started` | `{process_id, command}` | `rclone_cmds.rs` | `rclone_exec` çağrıldığında |
| `rclone:process-completed` | `{process_id}` | `rclone_cmds.rs` | Process çıkış yaptığında |
| `rclone:mount-status` | `{mount_id, status}` | `rclone_cmds.rs` | Mount başlatıldığında |
| `task:completed` | `{task_id, task_name, started_at, completed_at}` | `scheduler.rs` | Task başarıyla tamamlandı |
| `task:error` | `{task_id, task_name, error}` | `scheduler.rs` | Task hata ile sonuçlandı |
