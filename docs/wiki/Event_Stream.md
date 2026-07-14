# Event_Stream

**Özet:** Rclone süreçlerinin stdout/stderr çıktısını regex ile ayrıştırarak Tauri event'leri (`emit`) ile frontend'e gerçek zamanlı ileten pipeline. Hem manuel `rclone_exec` süreçleri hem de scheduler engine tarafından başlatılan task süreçleri aynı event mekanizmasını kullanır.

**Kütüphaneler:** tokio, regex, serde, serde_json

**Bağlantılar:** [[Tauri_Backend]], [[Process_Manager]], [[React_Frontend]], [[Rclone_Integration]]

---

## Gerçekleşen Pipeline

**Proje dosyası**: `src-tauri/src/rclone/events.rs`
**Scheduler engine**: `src-tauri/src/scheduler/engine.rs`

### Manuel Süreç (rclone_exec)

```
stdout/stderr (tokio::process::Command)
    │
    ▼
start_event_stream() → BufReader line-by-line
    │
    ├── parse_progress_line() → match → emit("rclone:progress", ProgressPayload)
    │
    └── no match → emit("rclone:log", { process_id, line })
    │
    ▼
event_handle.await → emit("rclone:process-completed", { process_id })
```

### Scheduler Task Süreci (execute_task)

```
stdout → BufReader → parse_progress_line() → match → emit("rclone:progress")
stderr → BufReader → emit("rclone:log")
    │
    ▼
child.wait() → exit code
    │
    ├── success → DB'ye transfers kaydı (status: "completed")
    │           → emit("task:completed", { task_id, task_name, ... })
    │
    └── error   → DB'ye transfers kaydı (status: "error")
                → emit("task:error", { task_id, error, ... })
```

## Regex Deseni

Regex `OnceLock` içinde bir kere derlenir (lazy_static yerine Rust 1.80+ standardı):

```rust
use std::sync::OnceLock;

fn progress_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"Transferred:\s+(?P<transferred>[\d.]+\s*\w*)\s+/\s+(?P<total>[\d.]+\s*\w*),\s+(?P<percent>\d+)%,\s+(?P<speed>[\d.]+\s*\w*/s)(?:,\s+ETA\s+(?P<eta>[\w\d-]+))?"
        ).expect("invalid regex")
    })
}
```

ETA alanı `[\w\d-]` ile genişletilmiştir — rclone bazen `ETA -` (bilinmiyor) döndürür.

## Event Payload'ları

### ProgressPayload (`events.rs`)

```rust
#[derive(Clone, Serialize)]
pub struct ProgressPayload {
    pub process_id: String,
    pub transferred: String,
    pub total: String,
    pub percent: u8,
    pub speed: String,
    pub eta: String,
}
```

### TaskResult (`engine.rs`)

```rust
#[derive(Debug, Clone, Serialize)]
pub struct TaskResult {
    pub task_id: String,
    pub process_id: String,
    pub started_at: String,
    pub completed_at: Option<String>,
    pub success: bool,
    pub error_message: Option<String>,
}
```

## Event Listesi (9 adet)

| Event | Payload | Açıklama |
|---|---|---|
| `rclone:progress` | `ProgressPayload` | İlerleme yüzdesi, hız, ETA |
| `rclone:process-started` | `{process_id, command}` | Yeni süreç başladı |
| `rclone:process-completed` | `{process_id}` | Süreç tamamlandı (exit sonrası) |
| `rclone:mount-status` | `{mount_id, status}` | Mount durum değişikliği |
| `rclone:log` | `{process_id, line}` | stdout/stderr log satırı |
| `rclone:binary-missing` | `{platform}` | Binary bulunamadı |
| `task:completed` | `{task_id, task_name, started_at, completed_at}` | Task başarılı |
| `task:error` | `{task_id, task_name, error}` | Task hatası |

## Frontend'de Dinleme (TransferPanel.tsx)

```typescript
import { listen } from "@tauri-apps/api/event";

useEffect(() => {
    let cancelled = false;
    const unlisten = listen<ProgressPayload>("rclone:progress", (event) => {
        if (cancelled) return;
        if (event.payload.process_id !== currentPidRef.current) return;
        setProgress(event.payload.percent);
        setSpeed(event.payload.speed);
    });
    return () => { cancelled = true; unlisten.then(fn => fn()); };
}, []);
```
