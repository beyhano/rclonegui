# 🕳️ Karadelik (Black Hole)

**Amaç:** Zamanlanmış görevlerde hedef olarak null cihaz. Dosyaları okur, `/dev/null`'a (Linux) veya `NUL`'a (Windows) yazar, iş biter.

---

## Nasıl Çalışır

### Backend (`scheduler/engine.rs`)

`dest_provider == "(karadelik)"` olunca:

```
Windows → NUL
Linux   → /dev/null
```

rclone argümanlarında hedef olarak null cihaz kullanılır. rclone dosyaları teker teker okur, doğrudan null cihaza yazar. Aracı yok, tampon yok, geçici klasör yok.

### Frontend — TaskFormModal

Destination dropdown'ında seçenek: **🕳️ Karadelik (Veri Yok Olur!)**

Seçilince:
- Path input'u ve Browse butonu gizlenir
- Kırmızı uyarı kutusu görünür
- SweetAlert2 onay dialog'u: checkbox işaretlenmeden kaydedilemez

### Frontend — TaskCard

`dest_provider === "(karadelik)"` olan görevlerde kırmızı `🕳️ Karadelik` badge'i.

---

## Uyarılar

| Operasyon | Kaynak | Hedef |
|-----------|--------|-------|
| **Copy** | Değişmez | `/dev/null` / `NUL` |
| **Move/Sync** | **Silinir** | `/dev/null` / `NUL` |

**Move + Karadelik = Kaynak dosyalar kalıcı yok olur.** Geri dönüşüm kutusuna gitmez.

---

## Test

1. Scheduler → Yeni Görev
2. Kaynak: `gdrive:/test` (veya herhangi bir remote)
3. Hedef: 🕳️ Karadelik
4. İşlem: Copy (güvenli)
5. "▶ Çalıştır" ile manuel tetikle
6. Progress bar'ı izle

---

## Dosyalar

| Katman | Dosya | Ne Yapıldı |
|--------|-------|-----------|
| Backend | `engine.rs` | `dest == "(karadelik)"` → `/dev/null` / `NUL` |
| Frontend | `TaskFormModal.tsx` | Dropdown + uyarı + SweetAlert2 |
| Frontend | `TaskCard.tsx` | Kırmızı badge |
| Bağımlılık | `package.json` | `sweetalert2` eklendi |
