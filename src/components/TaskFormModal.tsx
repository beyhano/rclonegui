import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import Swal from "sweetalert2";
import CronInput from "./CronInput";
import { Task, Remote, generateSlug } from "../types";

interface TaskFormData {
  name: string;
  slug: string;
  source: string;
  source_path: string;
  dest: string;
  dest_path: string;
  operation: string;
  exclude_patterns: string[];
  cron_expr: string;
}

interface Props {
  onClose: () => void;
  onCreated: (task: Task) => void;
  editTask?: Task;
}

function parsePath(fullPath: string): { remote: string; path: string } {
  if (fullPath === "(karadelik)") {
    return { remote: "(karadelik)", path: "" };
  }
  if (/^[A-Za-z]:[\\/]/.test(fullPath)) {
    return { remote: "local", path: fullPath };
  }
  const colonIdx = fullPath.indexOf(":");
  if (colonIdx > 0) {
    return {
      remote: fullPath.slice(0, colonIdx),
      path: fullPath.slice(colonIdx + 1),
    };
  }
  return { remote: "local", path: fullPath };
}

export default function TaskFormModal({ onClose, onCreated, editTask }: Props) {
  const [step, setStep] = useState(1);
  const [remotes, setRemotes] = useState<Remote[]>([]);
  const [form, setForm] = useState<TaskFormData>({
    name: "", slug: "", source: "local", source_path: "",
    dest: "local", dest_path: "", operation: "copy",
    exclude_patterns: [], cron_expr: "0 0 3 * * *",
  });
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState("");
  const [browsingRemote, setBrowsingRemote] = useState<{
    remote: string;
    path: string;
    onSelect: (path: string) => void;
  } | null>(null);

  useEffect(() => {
    invoke<Remote[]>("rclone_config_list").then(setRemotes).catch(console.error);
  }, []);

  useEffect(() => {
    if (editTask) {
      const src = parsePath(editTask.source_provider);
      const dst = parsePath(editTask.dest_provider);
      setForm({
        name: editTask.name,
        slug: editTask.slug,
        source: src.remote,
        source_path: src.path,
        dest: dst.remote,
        dest_path: dst.path,
        operation: editTask.operation,
        exclude_patterns: editTask.exclude_patterns,
        cron_expr: editTask.cron_expr,
      });
    }
  }, [editTask]);

  const updateName = (name: string) => {
    setForm(f => ({ ...f, name, slug: generateSlug(name) }));
  };

  const buildFullPath = (remote: string, path: string) => {
    if (remote === "local") return path;
    if (remote === "(karadelik)") return "(karadelik)";
    return path ? `${remote}:${path}` : `${remote}:`;
  };

  const sourceFull = buildFullPath(form.source, form.source_path);
  const destFull = buildFullPath(form.dest, form.dest_path);

  const canProceed = () => {
    switch (step) {
      case 1: return form.name.trim().length > 0;
      case 2: return sourceFull.length > 0 && destFull.length > 0;
      default: return true;
    }
  };

  const handleSubmit = async () => {
    // Karadelik onay dialog'u
    if (form.dest === "(karadelik)") {
      const result = await Swal.fire({
        title: "🚨 Veri Kaybı Uyarısı",
        html: `
          <div style="text-align:left;font-size:0.95rem;">
            <p style="color:#d32f2f;font-weight:bold;font-size:1.1rem;">
              Karadeliğe gönderilen dosyalar <u>kalıcı olarak yok olur</u>!
            </p>
            <ul style="margin-top:0.75rem;line-height:1.6;">
              <li><b>Copy</b> işlemi dosyaları okur ve atar, kaynakta bir şey değişmez.</li>
              <li><b>Sync/Move</b> işlemi kaynak dosyaları da <b>siler</b>.</li>
              <li>Bu işlem <b>geri alınamaz</b>.</li>
            </ul>
            <p style="margin-top:0.75rem;color:#555;">
              Sadece okuma/test amaçlı kullanın.
            </p>
          </div>
        `,
        input: "checkbox",
        inputPlaceholder: "Anladım, veri kaybını kabul ediyorum",
        confirmButtonText: "Evet, gönder",
        cancelButtonText: "İptal",
        showCancelButton: true,
        reverseButtons: true,
        allowOutsideClick: false,
        preConfirm: () => {
          const popup = Swal.getPopup();
          const cb = popup?.querySelector("#swal2-checkbox") as HTMLInputElement;
          if (!cb?.checked) {
            Swal.showValidationMessage("Veri kaybını kabul etmelisiniz");
          }
        },
      });
      if (!result.isConfirmed) return;
    }

    setSubmitting(true);
    setError("");
    try {
      let task: Task;
      if (editTask) {
        task = await invoke<Task>("task_update", {
          id: editTask.id,
          name: form.name,
          slug: form.slug,
          sourceProvider: sourceFull,
          sourceConfig: "{}",
          destProvider: destFull,
          destConfig: "{}",
          operation: form.operation,
          excludePatterns: form.exclude_patterns,
          cronExpr: form.cron_expr,
        });
      } else {
        task = await invoke<Task>("task_create", {
          name: form.name,
          sourceProvider: sourceFull,
          sourceConfig: "{}",
          destProvider: destFull,
          destConfig: "{}",
          operation: form.operation,
          excludePatterns: form.exclude_patterns,
          cronExpr: form.cron_expr,
        });
      }
      onCreated(task);
      onClose();
    } catch (e) {
      setError(String(e));
    } finally {
      setSubmitting(false);
    }
  };

  const RemotePathRow = ({ label, remote, path, setRemote, setPath, showBlackhole }: {
    label: string; remote: string; path: string;
    setRemote: (v: string) => void; setPath: (v: string) => void;
    showBlackhole?: boolean;
  }) => {
    const handleBrowse = async () => {
      if (remote === "local") {
        try {
          const selected = await open({
            directory: true,
            multiple: false,
          });
          if (selected && typeof selected === "string") {
            setPath(selected);
          }
        } catch (err) {
          console.error("Browse failed:", err);
        }
      } else {
        setBrowsingRemote({
          remote,
          path,
          onSelect: setPath,
        });
      }
    };

    return (
      <div className="remote-path-row">
        <label>{label}</label>
        <div className="remote-path-controls">
          <select value={remote} onChange={e => { setRemote(e.target.value); setPath(""); }}>
            <option value="local">📁 Yerel Klasör</option>
            {remotes.map(r => (
              <option key={r.name} value={r.name}>{r.name} ({r.type})</option>
            ))}
            {showBlackhole && <option value="(karadelik)">🕳️ Karadelik (Veri Yok Olur!)</option>}
          </select>
          {remote !== "(karadelik)" && (
            <>
              <input
                type="text"
                value={path}
                onChange={e => setPath(e.target.value)}
                placeholder={remote === "local" ? "C:\\Users\\... veya /home/..." : "alt klasör (isteğe bağlı)"}
                className="path-input"
              />
              <button type="button" onClick={handleBrowse} className="btn-browse" title={remote === "local" ? "Yerel klasör seç" : "Uzak klasör seç"}>
                📂
              </button>
            </>
          )}
        </div>
        {remote !== "(karadelik)" && (
          <code className="path-preview">{buildFullPath(remote, path) || "—"}</code>
        )}
      </div>
    );
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal" onClick={e => e.stopPropagation()}>
        <h2>{editTask ? `Görevi Düzenle — Adım ${step}/3` : `Yeni Görev — Adım ${step}/3`}</h2>
        
        {step === 1 && (
          <div className="modal-step">
            <label>Görev Adı</label>
            <input value={form.name} onChange={e => updateName(e.target.value)} autoFocus />
            <label>Slug (otomatik oluşturulur)</label>
            <input value={form.slug} onChange={e => setForm(f => ({ ...f, slug: e.target.value }))} />
          </div>
        )}

        {step === 2 && (
          <div className="modal-step">
            <RemotePathRow
              label="Kaynak"
              remote={form.source} path={form.source_path}
              setRemote={v => setForm(f => ({ ...f, source: v }))}
              setPath={v => setForm(f => ({ ...f, source_path: v }))}
            />
            <RemotePathRow
              label="Hedef"
              remote={form.dest} path={form.dest_path}
              setRemote={v => setForm(f => ({ ...f, dest: v }))}
              setPath={v => setForm(f => ({ ...f, dest_path: v }))}
              showBlackhole
            />
            {form.dest === "(karadelik)" && (
              <div style={{ background: "#fbe9e7", border: "1px solid #d32f2f", borderRadius: 6, padding: "0.75rem 1rem", marginTop: "0.5rem" }}>
                <p style={{ color: "#c62828", fontWeight: 600, fontSize: "0.9rem", margin: 0 }}>
                  ⚠️ Karadeliğe gönderilen dosyalar kalıcı olarak yok olur!
                </p>
                <p style={{ color: "#c62828", fontSize: "0.85rem", marginTop: "0.25rem", marginBottom: 0 }}>
                  Sync/Move kaynak dosyaları da siler. Copy kaynağa dokunmaz.
                  Sadece test amaçlı kullanın.
                </p>
              </div>
            )}
          </div>
        )}

        {step === 3 && (
          <div className="modal-step">
            <label>İşlem</label>
            <select value={form.operation} onChange={e => setForm(f => ({ ...f, operation: e.target.value }))}>
              <option value="copy">Kopyala</option>
              <option value="sync">Senkronize Et</option>
              <option value="move">Taşı</option>
              <option value="bisync">Çift Yönlü</option>
            </select>
            <label>Hariç Tutma Kalıpları (her satıra bir tane)</label>
            <textarea value={form.exclude_patterns.join("\n")} onChange={e => setForm(f => ({ ...f, exclude_patterns: e.target.value.split("\n").filter(Boolean) }))} placeholder="node_modules/&#10;*.tmp&#10;.git/**" />
            <CronInput value={form.cron_expr} onChange={v => setForm(f => ({ ...f, cron_expr: v }))} />
          </div>
        )}

        {error && <p className="error">{error}</p>}

        <div className="modal-actions">
          {step > 1 && <button onClick={() => setStep(s => s - 1)}>Geri</button>}
          {step < 3 ? (
            <button onClick={() => setStep(s => s + 1)} disabled={!canProceed()}>İleri</button>
          ) : (
            <button onClick={handleSubmit} disabled={submitting}>
              {submitting ? "Kaydediliyor..." : editTask ? "Görevi Güncelle" : "Görev Oluştur"}
            </button>
          )}
          <button onClick={onClose}>İptal</button>
        </div>
      </div>

      {browsingRemote && (
        <RemoteBrowserModal
          remote={browsingRemote.remote}
          initialPath={browsingRemote.path}
          onClose={() => setBrowsingRemote(null)}
          onSelect={browsingRemote.onSelect}
        />
      )}
    </div>
  );
}

interface RemoteBrowserModalProps {
  remote: string;
  initialPath: string;
  onClose: () => void;
  onSelect: (path: string) => void;
}

function RemoteBrowserModal({ remote, initialPath, onClose, onSelect }: RemoteBrowserModalProps) {
  const [path, setPath] = useState(initialPath);
  const [dirs, setDirs] = useState<string[]>([]);
  const [showHidden, setShowHidden] = useState(false);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState("");

  const loadDirs = async (currentPath: string) => {
    setLoading(true);
    setError("");
    try {
      const result = await invoke<string[]>("rclone_list_dirs", {
        remote,
        path: currentPath,
      });
      setDirs(result);
      setPath(currentPath);
    } catch (err) {
      setError(String(err));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    loadDirs(initialPath);
  }, [remote, initialPath]);

  const navigateTo = (sub: string) => {
    const nextPath = path ? `${path}/${sub}` : sub;
    loadDirs(nextPath);
  };

  const goUp = () => {
    if (!path) return;
    const parts = path.split("/");
    parts.pop();
    const nextPath = parts.join("/");
    loadDirs(nextPath);
  };

  const filteredDirs = dirs.filter(d => showHidden || !d.startsWith("."));

  return (
    <div className="modal-overlay" style={{ zIndex: 200 }} onClick={onClose}>
      <div className="modal remote-browser-modal" onClick={e => e.stopPropagation()}>
        <h3>Uzak Sunucu: {remote}</h3>
        <div className="path-display" style={{ margin: "0.5rem 0", display: "flex", alignItems: "center", gap: "0.5rem" }}>
          <span>Yol:</span>
          <code className="path-preview" style={{ flex: 1 }}>{path || "/"}</code>
        </div>

        <div style={{ display: "flex", alignItems: "center", gap: "0.5rem", margin: "0.5rem 0" }}>
          <input
            type="checkbox"
            id="show-hidden"
            checked={showHidden}
            onChange={e => setShowHidden(e.target.checked)}
            className="checkbox-input"
          />
          <label htmlFor="show-hidden" style={{ cursor: "pointer", fontSize: "0.9rem" }}>Gizli klasörleri göster</label>
        </div>

        {loading && <p style={{ padding: "1rem", textAlign: "center" }}>Klasörler yükleniyor...</p>}
        {error && <p className="error" style={{ padding: "1rem 0" }}>{error}</p>}

        {!loading && !error && (
          <div className="directory-list" style={{ maxHeight: "250px", overflowY: "auto", margin: "1rem 0" }}>
            {path && (
              <div className="directory-item up" onClick={goUp}>
                📁 .. (Yukarı Çık)
              </div>
            )}
            {filteredDirs.length === 0 ? (
              <div style={{ padding: "1rem", color: "#888", textAlign: "center" }}>Alt klasör bulunamadı</div>
            ) : (
              filteredDirs.map(d => (
                <div key={d} className="directory-item" onClick={() => navigateTo(d)}>
                  📁 {d}
                </div>
              ))
            )}
          </div>
        )}

        <div className="modal-actions" style={{ display: "flex", gap: "0.5rem", justifyContent: "flex-end" }}>
          <button className="btn-primary" onClick={() => { onSelect(path); onClose(); }}>Klasör Seç</button>
          <button onClick={onClose}>İptal</button>
        </div>
      </div>
    </div>
  );
}
