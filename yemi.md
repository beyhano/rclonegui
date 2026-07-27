# Yapılanlar — .gitignore Desteği

## Özet
RcloneGUI'ye **recursive .gitignore desteği** eklendi. Artık kaynak klasör seçildiğinde alt projelerin tüm `.gitignore` dosyaları taranır, pattern'ler rclone'a `--exclude-from <temp_dosya>` ile iletilir. Böylece her proje kendi `.gitignore`'una göre exclude edilir.

---

## 1. Rust Backend — `parse_gitignore` + `prepare_gitignore_excludes`

### `src-tauri/src/commands/rclone_cmds.rs`

**`collect_gitignore_recursive(root)`** — Ana fonksiyon:
- Verilen dizini recursive tara
- Her `.gitignore` dosyasını bul
- Pattern'leri oku (boş satır ve `#` yorumlarını atla)
- Pattern'leri rclone sözdizimine uygun genişlet:
  - Root `.gitignore` → `pattern`, `pattern/`, `pattern/**` (veya köklü `/pattern/**`)
  - `proj1/.gitignore` → `proj1/pattern`, `proj1/pattern/`, `proj1/pattern/**` ve alt dizinler için `proj1/**/pattern/**`
  - Dizin desenleri (`build/`, `dist`, `foo/bar`) rclone'da `dir/` ve `dir/**` varyasyonları ile hem dizin hem dosya kopyalama aşamasında tam hariç tutulur.
- `.` ile başlayan dizinler ve `node_modules/` atlanır (gereksiz tarama)

**`parse_gitignore(path)`** — Tauri command, pattern listesi döndürür (TaskFormModal için)

**`prepare_gitignore_excludes(path)`** — YENİ Tauri command:
- Pattern'leri topla
- `/tmp/rclonegui_excludes_<uuid>.txt` dosyasına yaz
- Dosya yolunu döndür
- TransferPanel bu yolu `--exclude-from` olarak kullanır

### `src-tauri/src/scheduler/engine.rs`

**`write_exclude_file(patterns, process_id)`** — Helper:
- Pattern'leri temp dosyaya yaz
- `--exclude-from` kullan (3 ayrı `--exclude` döngüsü kaldırıldı)
- Task bitince temp dosyayı sil

### `src-tauri/src/lib.rs`

- `parse_gitignore` ve `prepare_gitignore_excludes` command olarak register edildi

---

## 2. Frontend — TransferPanel

### `src/TransferPanel.tsx`

- **🔒 butonu** source input'un yanında
- Tıklayınca:
  1. `parse_gitignore` → pattern listesi (badge'de gösterilir: "N desen yüklendi")
  2. `prepare_gitignore_excludes` → temp dosya yolu (ref'te saklanır)
- **Start Transfer**:
  - `--exclude-from <temp_dosya>` args'a eklenir (artık tek tek `--exclude` flag'leri değil)
- Source değişince pattern'ler ve temp dosya referansı sıfırlanır

---

## 3. Frontend — TaskFormModal

### `src/components/TaskFormModal.tsx`

- Step 3'te **📥 .gitignore** butonu
- Sadece **local** kaynak seçiliyken görünür
- Tıklayınca `parse_gitignore(source_path)` çağır
- Gelen pattern'ler mevcut exclude listesine **merge** edilir (Set ile tekrarsız)
- Kullanıcı textarea'da düzenleyebilir, silebilir
- Task kaydedilince DB'ye `exclude_patterns` JSON olarak yazılır
- Task çalışınca `engine.rs` `--exclude-from` ile temp dosyadan okur

---

## 4. CSS

### `src/App.css`

- `.input-with-button` — Source input + 🔒 buton yanyana
- `.btn-gitignore`, `.btn-gitignore-sm` — Buton stilleri (light + dark mode)
- `.gitignore-badge` — "N desen yüklendi" metni (yeşil)

---

## 5. Testler (5 adet)

Hepsi `commands::rclone_cmds::tests` modülünde:

| Test | Ne kontrol eder |
|------|----------------|
| `test_parse_gitignore_file_not_found_returns_empty` | Dosya yoksa boş liste |
| `test_parse_gitignore_skips_blanks_and_comments` | Boş satır/yorum atlanır |
| `test_parse_gitignore_returns_all_valid_patterns` | Root-level pattern'ler olduğu gibi gelir |
| `test_parse_gitignore_recursive_subdirs` | Subdir pattern'leri prefix'lenir, ** variantları eklenir |
| `test_parse_gitignore_skips_hidden_and_node_dirs` | `.hidden/` ve `node_modules/` atlanır |

**Toplam: 71 test, 0 hata**

---

## 6. CHANGELOG.md

`[0.1.10] — 2026-07-27` eklendi: .gitignore desteği, düzeltmeler.

---

## 7. Kritik Tespit: `--exclude` vs `--exclude-from`

**Öncesi (çalışmadı):** Her pattern ayrı `--exclude` flag'i olarak → 2131 pattern = 4262 argüman. rclone boğuluyordu, hiçbir şey exclude edilmiyordu.

**Sonrası (çalışıyor):** Tüm pattern'ler tek dosyada → `rclone --exclude-from /tmp/patterns.txt`

Test ile kanıtlandı:
- 9 dosyalı test dizininde filtresiz: 9/9 dosya aktarıldı
- `--exclude-from` ile: 4/4 (sadece kaynak dosyalar) — node_modules, build, .cache, .env, .log hepsi dışlandı ✅

---

## 8. Nasıl Çalışır (Akış)

```
Kullanıcı source path girer → 🔒 tıklar
  → Backend: recursive .gitignore tara
  → Pattern'leri prefix'le + temp dosyaya yaz
  → Frontend: badge göster, temp dosya yolunu sakla
→ Kullanıcı Start Transfer tıklar
  → Frontend: args = ["copy", source, dest, "--exclude-from", temp_path, "--progress"]
  → Backend (rclone_exec): rclone'u bu arg'larla çalıştır
  → rclone: temp dosyadaki pattern'lere göre exclude et
```

---

## 9. İleride

- TaskFormModal için "şu anda hangi pattern'ler aktif" özeti
- Manuel transferde `.gitignore` yüklenmeden transfer başlatılırsa uyarı
- Çok büyük projelerde (100K+ pattern) performans iyileştirmesi
