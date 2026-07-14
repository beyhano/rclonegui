import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
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

  const RemotePathRow = ({ label, remote, path, setRemote, setPath }: {
    label: string; remote: string; path: string;
    setRemote: (v: string) => void; setPath: (v: string) => void;
  }) => {
    const handleBrowse = async () => {
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
    };

    return (
      <div className="remote-path-row">
        <label>{label}</label>
        <div className="remote-path-controls">
          <select value={remote} onChange={e => setRemote(e.target.value)}>
            <option value="local">📁 Local folder</option>
            {remotes.map(r => (
              <option key={r.name} value={r.name}>{r.name} ({r.type})</option>
            ))}
          </select>
          <input
            type="text"
            value={path}
            onChange={e => setPath(e.target.value)}
            placeholder={remote === "local" ? "C:\\Users\\... or /home/..." : "subfolder (optional)"}
            className="path-input"
          />
          {remote === "local" && (
            <button type="button" onClick={handleBrowse} className="btn-browse" title="Browse local directory">
              📂
            </button>
          )}
        </div>
        <code className="path-preview">{buildFullPath(remote, path) || "—"}</code>
      </div>
    );
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal" onClick={e => e.stopPropagation()}>
        <h2>{editTask ? `Edit Task — Step ${step}/3` : `New Task — Step ${step}/3`}</h2>
        
        {step === 1 && (
          <div className="modal-step">
            <label>Task Name</label>
            <input value={form.name} onChange={e => updateName(e.target.value)} autoFocus />
            <label>Slug (auto-generated)</label>
            <input value={form.slug} onChange={e => setForm(f => ({ ...f, slug: e.target.value }))} />
          </div>
        )}

        {step === 2 && (
          <div className="modal-step">
            <RemotePathRow
              label="Source"
              remote={form.source} path={form.source_path}
              setRemote={v => setForm(f => ({ ...f, source: v }))}
              setPath={v => setForm(f => ({ ...f, source_path: v }))}
            />
            <RemotePathRow
              label="Destination"
              remote={form.dest} path={form.dest_path}
              setRemote={v => setForm(f => ({ ...f, dest: v }))}
              setPath={v => setForm(f => ({ ...f, dest_path: v }))}
            />
          </div>
        )}

        {step === 3 && (
          <div className="modal-step">
            <label>Operation</label>
            <select value={form.operation} onChange={e => setForm(f => ({ ...f, operation: e.target.value }))}>
              <option value="copy">Copy</option>
              <option value="sync">Sync</option>
              <option value="move">Move</option>
              <option value="bisync">Bisync</option>
            </select>
            <label>Exclude Patterns (one per line)</label>
            <textarea value={form.exclude_patterns.join("\n")} onChange={e => setForm(f => ({ ...f, exclude_patterns: e.target.value.split("\n").filter(Boolean) }))} placeholder="node_modules/&#10;*.tmp&#10;.git/**" />
            <CronInput value={form.cron_expr} onChange={v => setForm(f => ({ ...f, cron_expr: v }))} />
          </div>
        )}

        {error && <p className="error">{error}</p>}

        <div className="modal-actions">
          {step > 1 && <button onClick={() => setStep(s => s - 1)}>Back</button>}
          {step < 3 ? (
            <button onClick={() => setStep(s => s + 1)} disabled={!canProceed()}>Next</button>
          ) : (
            <button onClick={handleSubmit} disabled={submitting}>
              {submitting ? "Saving..." : editTask ? "Update Task" : "Create Task"}
            </button>
          )}
          <button onClick={onClose}>Cancel</button>
        </div>
      </div>
    </div>
  );
}
