import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import ProviderSelector from "./ProviderSelector";
import ProviderConfigForm from "./ProviderConfigForm";
import CronInput from "./CronInput";
import { Task, Provider, generateSlug } from "../types";

interface TaskFormData {
  name: string;
  slug: string;
  source_provider: string;
  source_config: Record<string, string>;
  dest_provider: string;
  dest_config: Record<string, string>;
  operation: string;
  exclude_patterns: string[];
  cron_expr: string;
}

interface Props {
  onClose: () => void;
  onCreated: (task: Task) => void;
}

export default function TaskFormModal({ onClose, onCreated }: Props) {
  const [step, setStep] = useState(1);
  const [providers, setProviders] = useState<Provider[]>([]);
  const [form, setForm] = useState<TaskFormData>({
    name: "", slug: "", source_provider: "", source_config: {},
    dest_provider: "", dest_config: {}, operation: "copy",
    exclude_patterns: [], cron_expr: "0 0 3 * * *",
  });
  const [submitting, setSubmitting] = useState(false);
  const [error, setError] = useState("");

  useEffect(() => {
    invoke<Provider[]>("rclone_providers").then(setProviders).catch(console.error);
  }, []);

  const updateName = (name: string) => {
    setForm(f => ({ ...f, name, slug: generateSlug(name) }));
  };

  const getProviderOptions = (prefix: string) => {
    const p = providers.find(p => p.Prefix === prefix);
    return p?.Options ?? [];
  };

  const canProceed = () => {
    switch (step) {
      case 1: return form.name.trim().length > 0;
      case 2: return form.source_provider && form.dest_provider;
      default: return true;
    }
  };

  const handleSubmit = async () => {
    setSubmitting(true);
    setError("");
    try {
      const task = await invoke<Task>("task_create", {
        name: form.name,
        sourceProvider: form.source_provider,
        sourceConfig: JSON.stringify(form.source_config),
        destProvider: form.dest_provider,
        destConfig: JSON.stringify(form.dest_config),
        operation: form.operation,
        excludePatterns: form.exclude_patterns,
        cronExpr: form.cron_expr,
      });
      onCreated(task);
      onClose();
    } catch (e) {
      setError(String(e));
    } finally {
      setSubmitting(false);
    }
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal" onClick={e => e.stopPropagation()}>
        <h2>New Task — Step {step}/5</h2>
        
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
            <ProviderSelector label="Source Provider" value={form.source_provider} onChange={v => setForm(f => ({ ...f, source_provider: v }))} />
            <ProviderSelector label="Destination Provider" value={form.dest_provider} onChange={v => setForm(f => ({ ...f, dest_provider: v }))} />
          </div>
        )}

        {step === 3 && (
          <div className="modal-step">
            <h3>Source Config</h3>
            <ProviderConfigForm options={getProviderOptions(form.source_provider)} values={form.source_config} onChange={(k, v) => setForm(f => ({ ...f, source_config: { ...f.source_config, [k]: v } }))} />
            <h3>Dest Config</h3>
            <ProviderConfigForm options={getProviderOptions(form.dest_provider)} values={form.dest_config} onChange={(k, v) => setForm(f => ({ ...f, dest_config: { ...f.dest_config, [k]: v } }))} />
          </div>
        )}

        {step === 4 && (
          <div className="modal-step">
            <label>Operation</label>
            <select value={form.operation} onChange={e => setForm(f => ({ ...f, operation: e.target.value }))}>
              <option value="copy">Copy</option>
              <option value="sync">Sync</option>
              <option value="move">Move</option>
              <option value="bisync">Bisync</option>
            </select>
            <label>Exclude Patterns (one per line)</label>
            <textarea value={form.exclude_patterns.join("\n")} onChange={e => setForm(f => ({ ...f, exclude_patterns: e.target.value.split("\n").filter(Boolean) }))} />
          </div>
        )}

        {step === 5 && (
          <div className="modal-step">
            <CronInput value={form.cron_expr} onChange={v => setForm(f => ({ ...f, cron_expr: v }))} />
          </div>
        )}

        {error && <p className="error">{error}</p>}

        <div className="modal-actions">
          {step > 1 && <button onClick={() => setStep(s => s - 1)}>Back</button>}
          {step < 5 ? (
            <button onClick={() => setStep(s => s + 1)} disabled={!canProceed()}>Next</button>
          ) : (
            <button onClick={handleSubmit} disabled={submitting}>
              {submitting ? "Creating..." : "Create Task"}
            </button>
          )}
          <button onClick={onClose}>Cancel</button>
        </div>
      </div>
    </div>
  );
}
