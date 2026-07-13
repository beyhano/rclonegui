# Frontend Tasks 10-17: All UI components

This brief bundles all frontend tasks (10 through 17) since they're all independent TypeScript/React files in the `src/` directory. Complete them in order.

Working directory: `C:\Users\Beyhan\Desktop\Projeler\Rust\rclonegui`

---

## Task 10: Frontend types + slug util

**File:** `src/types.ts` (modify)

### Add these types:

```typescript
export interface Task {
  id: string;
  name: string;
  slug: string;
  source_provider: string;
  source_config: Record<string, unknown>;
  dest_provider: string;
  dest_config: Record<string, unknown>;
  operation: "copy" | "sync" | "move" | "bisync";
  exclude_patterns: string[];
  cron_expr: string;
  enabled: boolean;
  created_at: string;
  updated_at: string;
}

export interface ProviderOption {
  Name: string;
  Help: string;
  Default: unknown;
  Required: boolean;
  Type: string;
  Advanced: boolean;
  IsPassword: boolean;
  Exclusive: boolean;
  Examples: Array<{ Value: string; Help: string }> | null;
}

export interface Provider {
  Name: string;
  Description: string;
  Prefix: string;
  Options: ProviderOption[];
}
```

### Add slug utility function:

```typescript
export function generateSlug(name: string): string {
  return name
    .toLowerCase()
    .replace(/[şŞ]/g, 's')
    .replace(/[ıIİ]/g, 'i')
    .replace(/[üÜ]/g, 'u')
    .replace(/[öÖ]/g, 'o')
    .replace(/[çÇ]/g, 'c')
    .replace(/[ğĞ]/g, 'g')
    .replace(/[\s_]+/g, '-')
    .replace(/[^a-z0-9-]/g, '-')
    .replace(/-+/g, '-')
    .replace(/^-+|-+$/g, '');
}
```

**Verify:** `pnpm build` should pass.

**Commit:** `git add -A && git commit -m "feat(types): add Task, Provider types and slug util"`

---

## Task 11: ProviderSelector component

**File:** Create `src/components/ProviderSelector.tsx`

```tsx
import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

interface Provider {
  Name: string;
  Description: string;
  Prefix: string;
}

interface Props {
  value: string;
  onChange: (prefix: string) => void;
  label: string;
}

export default function ProviderSelector({ value, onChange, label }: Props) {
  const [providers, setProviders] = useState<Provider[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    invoke<Provider[]>("rclone_providers")
      .then(setProviders)
      .catch(console.error)
      .finally(() => setLoading(false));
  }, []);

  return (
    <div className="provider-selector">
      <label>{label}</label>
      {loading ? (
        <select disabled><option>Loading providers...</option></select>
      ) : (
        <select value={value} onChange={e => onChange(e.target.value)}>
          <option value="">-- Select provider --</option>
          {providers.map(p => (
            <option key={p.Prefix} value={p.Prefix}>
              {p.Name} — {p.Description}
            </option>
          ))}
        </select>
      )}
    </div>
  );
}
```

**Commit:** `git add -A && git commit -m "feat(ui): add ProviderSelector component"`

---

## Task 12: ProviderConfigForm (dynamic form)

**File:** Create `src/components/ProviderConfigForm.tsx`

```tsx
import { useState } from "react";

interface ProviderOption {
  Name: string;
  Help: string;
  Default: unknown;
  Required: boolean;
  Type: string;
  Advanced: boolean;
  IsPassword: boolean;
  Exclusive: boolean;
  Examples: Array<{ Value: string; Help: string }> | null;
}

interface Props {
  options: ProviderOption[];
  values: Record<string, string>;
  onChange: (name: string, value: string) => void;
}

export default function ProviderConfigForm({ options, values, onChange }: Props) {
  const basicOptions = options.filter(o => !o.Advanced);
  const advancedOptions = options.filter(o => o.Advanced);

  const renderField = (opt: ProviderOption) => {
    const value = values[opt.Name] ?? (opt.Default as string) ?? "";

    if (opt.Type === "bool") {
      return (
        <label key={opt.Name} className="config-field config-field--bool">
          <input
            type="checkbox"
            checked={value === "true"}
            onChange={e => onChange(opt.Name, e.target.checked ? "true" : "false")}
          />
          <span>{opt.Help}</span>
        </label>
      );
    }

    if (opt.Exclusive && opt.Examples && opt.Examples.length > 0) {
      return (
        <div key={opt.Name} className="config-field">
          <label>{opt.Help}{opt.Required && " *"}</label>
          <select value={value} onChange={e => onChange(opt.Name, e.target.value)}>
            <option value="">-- Select --</option>
            {opt.Examples.map(ex => (
              <option key={ex.Value} value={ex.Value}>{ex.Help} ({ex.Value})</option>
            ))}
          </select>
        </div>
      );
    }

    return (
      <div key={opt.Name} className="config-field">
        <label>{opt.Help}{opt.Required && " *"}</label>
        <input
          type={opt.IsPassword ? "password" : "text"}
          value={value}
          onChange={e => onChange(opt.Name, e.target.value)}
          placeholder={typeof opt.Default === "string" ? opt.Default : ""}
        />
      </div>
    );
  };

  if (options.length === 0) {
    return <p className="config-empty">Select a provider to see its options.</p>;
  }

  return (
    <div className="provider-config-form">
      {basicOptions.map(renderField)}
      {advancedOptions.length > 0 && (
        <details className="config-advanced">
          <summary>Advanced Options ({advancedOptions.length})</summary>
          {advancedOptions.map(renderField)}
        </details>
      )}
    </div>
  );
}
```

**Commit:** `git add -A && git commit -m "feat(ui): add ProviderConfigForm with dynamic fields"`

---

## Task 13: CronInput component

**File:** Create `src/components/CronInput.tsx`

```tsx
interface Props {
  value: string;
  onChange: (expr: string) => void;
}

const PRESETS = [
  { label: "Every hour", value: "0 0 * * * *" },
  { label: "Every 6 hours", value: "0 0 */6 * * *" },
  { label: "Daily at midnight", value: "0 0 0 * * *" },
  { label: "Daily at 03:00", value: "0 0 3 * * *" },
  { label: "Weekly (Mon 03:00)", value: "0 0 3 * * 1" },
  { label: "Monthly (1st 03:00)", value: "0 0 3 1 * *" },
];

export default function CronInput({ value, onChange }: Props) {
  return (
    <div className="cron-input">
      <label>Schedule (cron expression)</label>
      <input
        type="text"
        value={value}
        onChange={e => onChange(e.target.value)}
        placeholder="0 3 * * * *"
      />
      <div className="cron-presets">
        {PRESETS.map(p => (
          <button
            key={p.value}
            type="button"
            className={`cron-preset ${value === p.value ? "active" : ""}`}
            onClick={() => onChange(p.value)}
          >
            {p.label}
          </button>
        ))}
      </div>
    </div>
  );
}
```

**Commit:** `git add -A && git commit -m "feat(ui): add CronInput component with presets"`

---

## Task 14: TaskFormModal (5-step wizard)

**File:** Create `src/components/TaskFormModal.tsx`

```tsx
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
```

**Commit:** `git add -A && git commit -m "feat(ui): add TaskFormModal with 5-step wizard"`

---

## Task 15: TaskCard component

**File:** Create `src/components/TaskCard.tsx`

```tsx
import { Task } from "../types";

interface Props {
  task: Task;
  onToggle: (id: string) => void;
  onDelete: (id: string) => void;
  onRunNow: (id: string) => void;
}

export default function TaskCard({ task, onToggle, onDelete, onRunNow }: Props) {
  return (
    <div className={`task-card ${task.enabled ? "enabled" : "disabled"}`}>
      <div className="task-header">
        <h3>{task.name}</h3>
        <span className="task-slug">{task.slug}</span>
      </div>
      <div className="task-details">
        <span className="task-operation">{task.operation}</span>
        <span className="task-providers">{task.source_provider} → {task.dest_provider}</span>
        <span className="task-schedule">Cron: {task.cron_expr}</span>
      </div>
      <div className="task-actions">
        <button onClick={() => onRunNow(task.id)} title="Run Now">▶</button>
        <button onClick={() => onToggle(task.id)}>{task.enabled ? "⏸ Pause" : "▶ Resume"}</button>
        <button onClick={() => onDelete(task.id)}>Delete</button>
      </div>
    </div>
  );
}
```

**Commit:** `git add -A && git commit -m "feat(ui): add TaskCard component"`

---

## Task 16: SchedulerPage

**File:** Create `src/components/SchedulerPage.tsx`

```tsx
import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import TaskCard from "./TaskCard";
import TaskFormModal from "./TaskFormModal";
import { Task } from "../types";

export default function SchedulerPage() {
  const [tasks, setTasks] = useState<Task[]>([]);
  const [showForm, setShowForm] = useState(false);
  const [loading, setLoading] = useState(true);

  const loadTasks = () => {
    invoke<Task[]>("task_list")
      .then(setTasks)
      .catch(console.error)
      .finally(() => setLoading(false));
  };

  useEffect(() => {
    loadTasks();
    const unlisten1 = listen("task:completed", () => loadTasks());
    const unlisten2 = listen("task:error", () => loadTasks());
    return () => { unlisten1.then(f => f()); unlisten2.then(f => f()); };
  }, []);

  const handleToggle = async (id: string) => {
    await invoke("task_toggle", { id });
    loadTasks();
  };

  const handleDelete = async (id: string) => {
    await invoke("task_delete", { id });
    loadTasks();
  };

  const handleRunNow = async (id: string) => {
    await invoke("task_run_now", { id });
    loadTasks();
  };

  if (loading) return <div className="scheduler-page"><p>Loading tasks...</p></div>;

  return (
    <div className="scheduler-page">
      <div className="scheduler-header">
        <h2>Scheduled Tasks</h2>
        <button onClick={() => setShowForm(true)}>+ New Task</button>
      </div>
      {tasks.length === 0 ? (
        <div className="empty-state">
          <p>No tasks defined yet.</p>
          <p>Click "New Task" to create your first scheduled operation.</p>
        </div>
      ) : (
        <div className="task-list">
          {tasks.map(task => (
            <TaskCard key={task.id} task={task} onToggle={handleToggle} onDelete={handleDelete} onRunNow={handleRunNow} />
          ))}
        </div>
      )}
      {showForm && <TaskFormModal onClose={() => setShowForm(false)} onCreated={() => loadTasks()} />}
    </div>
  );
}
```

**Commit:** `git add -A && git commit -m "feat(ui): add SchedulerPage with task list"`

---

## Task 17: Scheduler tab + CSS

### Modify `src/App.tsx`

Add import:
```tsx
import SchedulerPage from "./components/SchedulerPage";
```

Change Tab type:
```tsx
type Tab = "config" | "transfer" | "mounts" | "scheduler";
```

Add button after the mounts button:
```tsx
<button
  className={`tab ${activeTab === "scheduler" ? "active" : ""}`}
  onClick={() => setActiveTab("scheduler")}
>Scheduler</button>
```

Add route:
```tsx
{activeTab === "scheduler" && <SchedulerPage />}
```

### Add CSS to `src/App.css`

Append at the end:
```css
/* Scheduler */
.scheduler-page { padding: 1rem; }
.scheduler-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 1rem; }
.scheduler-header h2 { margin: 0; }
.task-list { display: flex; flex-direction: column; gap: 0.75rem; }
.task-card { border: 1px solid #ccc; border-radius: 8px; padding: 1rem; background: #fff; }
.task-card.disabled { opacity: 0.6; }
.task-header { display: flex; justify-content: space-between; align-items: center; margin-bottom: 0.5rem; }
.task-header h3 { margin: 0; }
.task-slug { color: #888; font-size: 0.85rem; font-family: monospace; }
.task-details { display: flex; gap: 1rem; margin-bottom: 0.5rem; font-size: 0.9rem; color: #555; }
.task-actions { display: flex; gap: 0.5rem; }
.modal-overlay { position: fixed; inset: 0; background: rgba(0,0,0,0.5); display: flex; align-items: center; justify-content: center; z-index: 100; }
.modal { background: #fff; border-radius: 12px; padding: 2rem; min-width: 90%; max-width: 500px; max-height: 80vh; overflow-y: auto; }
.modal h2 { margin-top: 0; }
.modal-step { display: flex; flex-direction: column; gap: 1rem; }
.modal-step label { font-weight: 600; }
.modal-step input, .modal-step select, .modal-step textarea { padding: 0.5rem; border: 1px solid #ccc; border-radius: 6px; font-size: 1rem; }
.modal-step textarea { min-height: 80px; font-family: monospace; }
.modal-actions { display: flex; gap: 0.5rem; justify-content: flex-end; margin-top: 1.5rem; }
.provider-config-form { display: flex; flex-direction: column; gap: 1rem; }
.config-field { display: flex; flex-direction: column; gap: 0.25rem; }
.config-field--bool { flex-direction: row; align-items: center; }
.config-field input[type="checkbox"] { width: auto; }
.config-advanced { border: 1px dashed #ccc; padding: 0.75rem; border-radius: 6px; margin-top: 0.5rem; }
.config-empty { color: #888; font-style: italic; }
.cron-input { display: flex; flex-direction: column; gap: 0.75rem; }
.cron-presets { display: flex; flex-wrap: wrap; gap: 0.5rem; }
.cron-preset { padding: 0.35rem 0.75rem; border: 1px solid #ccc; border-radius: 6px; background: #f5f5f5; cursor: pointer; font-size: 0.85rem; }
.cron-preset.active { background: #0078d4; color: #fff; border-color: #0078d4; }
.error { color: #d32f2f; font-size: 0.9rem; }
.empty-state { text-align: center; color: #888; padding: 3rem; }
.btn-primary { background: #0078d4; color: #fff; border: none; padding: 0.5rem 1rem; border-radius: 6px; cursor: pointer; }
.btn-primary:hover { background: #005a9e; }

@media (prefers-color-scheme: dark) {
  .task-card { border-color: #444; background: #1e1e1e; }
  .task-slug { color: #999; }
  .task-details { color: #aaa; }
  .modal { background: #1e1e1e; }
  .modal-step input, .modal-step select, .modal-step textarea { background: #333; color: #eee; border-color: #555; }
  .cron-preset { background: #333; color: #eee; border-color: #555; }
  .cron-preset.active { background: #0078d4; color: #fff; }
  .config-advanced { border-color: #555; }
  .modal-overlay { background: rgba(0,0,0,0.7); }
}
```

### Verify

```bash
pnpm build
```
Expected: PASS (no type errors, no build errors)

**Commit:** `git add -A && git commit -m "feat(ui): add Scheduler tab and scheduler styles"`
