# Rclone_Integration

**Özet:** Sistemde kurulu rclone binary'sini keşfeden, versiyonunu doğrulayan ve `config`, `copy/sync`, `mount` komutlarını yöneten katman. Tüm bileşenler aktiftir ve çalışır durumdadır.

**Kütüphaneler:** tokio (process, io-util, sync), regex, serde, serde_json, uuid, chrono, rusqlite

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
└── rclone_cmds.rs   ← 7 adet #[tauri::command]
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

## Tauri Komutları (7 adet)

Tüm komutlar `commands/rclone_cmds.rs` içinde tanımlıdır ve `lib.rs`'de `invoke_handler` ile kaydedilmiştir.

### 1. `rclone_version`
```rust
#[tauri::command]
pub async fn rclone_version(state: State<'_, AppState>) -> Result<String, String>
```
- **İşlev**: `rclone version` çalıştırır, çıktıyı string olarak döner
- **Dönüş**: `"rclone v1.65.0\n..."` (ham stdout)
- **Hata**: Binary yoksa veya çalışmazsa hata mesajı

### 2. `rclone_config_list`
```rust
#[tauri::command]
pub async fn rclone_config_list(state: State<'_, AppState>) -> Result<Vec<Remote>, String>
```
- **İşlev**: `rclone config dump` JSON çıktısını parse eder
- **Dönüş**: `Vec<Remote>` — her remote için `{ name, type }`
- **Güvenlik**: Hassas alanlar (token, secret) otomatik atılır, yalnızca ad ve tür döner
- **Sıralama**: Remote'lar isme göre alfabetik sıralanır

### 3. `rclone_exec`
```rust
#[tauri::command]
pub async fn rclone_exec(
    app: AppHandle,
    state: State<'_, AppState>,
    args: Vec<String>,
) -> Result<String, String>
```
- **İşlev**: Rastgele rclone argümanları ile süreç başlatır
- **Dönüş**: UUID string — süreci takip etmek için
- **Yan etki**:
  - `rclone:process-started` event'i emit eder
  - stdout/stderr → `start_event_stream()` ile background task başlatır
  - Süreç `ProcessHandle` olarak `state.processes`'e kaydedilir

### 4. `rclone_stop`
```rust
#[tauri::command]
pub async fn rclone_stop(state: State<'_, AppState>, process_id: String) -> Result<(), String>
```
- **İşlev**: UUID ile process'i bulur, `start_kill()` gönderir
- **Hata**: ID bulunamazsa "Process not found"

### 5. `rclone_mount`
```rust
#[tauri::command]
pub async fn rclone_mount(
    app: AppHandle,
    state: State<'_, AppState>,
    remote: String,
    mount_point: String,
) -> Result<String, String>
```
- **İşlev**: `rclone mount <remote>: <mount_point>` başlatır
- **Dönüş**: UUID string (mount ID)
- **Yan etki**: `state.mounts`'a `MountInfo` kaydedilir, `rclone:mount-status` event'i emit edilir
- **Not**: Remote adı iki nokta üst üste içermiyorsa otomatik eklenir

### 6. `rclone_unmount`
```rust
#[tauri::command]
pub async fn rclone_unmount(state: State<'_, AppState>, mount_id: String) -> Result<(), String>
```
- **İşlev**: Mount process'ini durdurur ve `state.mounts`'tan kaydı temizler
- **Hata**: ID bulunamazsa hata

### 7. `rclone_mount_list`
```rust
#[tauri::command]
pub fn rclone_mount_list(state: State<'_, AppState>) -> Result<Vec<MountInfo>, String>
```
- **İşlev**: Tüm aktif mount'ların listesini döner
- **Dönüş**: `Vec<MountInfo>` — her mount için `{ id, remote, mount_point, status }`

## Event Türleri (6 adet)

Tüm event'ler `rclone:` namespace'i altında emit edilir. Frontend `listen()` ile yakalar.

| Event | Payload | Kaynak | Tetikleyici |
|---|---|---|---|
| `rclone:progress` | `{ process_id, transferred, total, percent, speed, eta }` | `events.rs` | Progress satırı eşleştiğinde |
| `rclone:log` | `{ process_id, line }` | `events.rs` | Her stdout/stderr satırı (progress dışı) |
| `rclone:process-started` | `{ process_id, command }` | `rclone_cmds.rs` | `rclone_exec` çağrıldığında |
| `rclone:process-completed` | `{ process_id, exit_code }` | Frontend bekler | Process çıkış yaptığında (planlanan) |
| `rclone:process-error` | `{ process_id, exit_code, stderr_lines }` | Frontend bekler | Process hata ile çıktığında (planlanan) |
| `rclone:mount-status` | `{ mount_id, status }` | `rclone_cmds.rs` | Mount başlatıldığında |

### Progress Regex

```rust
fn progress_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"Transferred:\s+(?P<transferred>[\d.]+\s*\w*)\s+/\s+(?P<total>[\d.]+\s*\w*),\s+(?P<percent>\d+)%,\s+(?P<speed>[\d.]+\s*\w+/s)(?:,\s+ETA\s+(?P<eta>[\w\d-]+))?",
        )
        .expect("valid progress regex")
    })
}
```

### ProgressPayload Yapısı

```rust
#[derive(Debug, Clone, Serialize)]
pub struct ProgressPayload {
    pub process_id: String,
    pub transferred: String,   // "1.190 GiB"
    pub total: String,         // "1.190 GiB"
    pub percent: u8,           // 100
    pub speed: String,         // "12.034 MiB/s"
    pub eta: String,           // "0s"
}
```

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

## Veri Akış Şeması

```
Frontend invoke
    │
    ▼
rclone_cmds.rs (Tauri komutu)
    │
    ├── discovery.rs → binary yolunu al
    ├── config.rs → rclone config dump (config listesi)
    ├── process.rs → spawn / stop (süreç yönetimi)
    └── events.rs → event stream başlat
        │
        ▼
    app_handle.emit("rclone:progress", payload)
        │
        ▼
    Frontend listen("rclone:progress") → UI güncellemesi
```

## Binary Path Güvenliği

- `discovery.rs` içinde `verify_executable()` ile doğrulama:
  - Unix: `mode & 0o111` executable bit kontrolü
  - Windows: `.exe` uzantı kontrolü
  - Yoksa: "Binary not found" hatası
- `state.rclone_path` `Arc<Mutex<Option<PathBuf>>>` ile thread-safe
- `setup()` anında `resource_dir/rclone-bin/{platform}/` otomatik taranır
- Binary bulunamazsa tüm komutlar güvenli hata döner (panic yok)
