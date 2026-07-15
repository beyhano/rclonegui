import { Task } from "../types";

interface Props {
  task: Task;
  onToggle: (id: string) => void;
  onDelete: (id: string) => void;
  onRunNow: (id: string) => void;
  onEdit: (task: Task) => void;
  onStop: (id: string) => void;
  isRunning?: boolean;
  progress?: { percent: number; speed: string; eta: string };
}

export default function TaskCard({ task, onToggle, onDelete, onRunNow, onEdit, onStop, isRunning, progress }: Props) {
  return (
    <div className={`task-card ${task.enabled ? "enabled" : "disabled"}`}>
      <div className="task-header">
        <h3>{task.name}</h3>
        <span className="task-slug">{task.slug}</span>
        {task.dest_provider === "(karadelik)" && (
          <span style={{ background: "#c62828", color: "#fff", borderRadius: 4, padding: "0.15rem 0.6rem", fontSize: "0.75rem", fontWeight: 600, marginLeft: "auto" }}>
            🕳️ Karadelik
          </span>
        )}
      </div>
      <div className="task-details">
        <span className="task-operation">{task.operation}</span>
        <span className="task-providers">{task.source_provider} → {task.dest_provider}</span>
        <span className="task-schedule">Saat: {task.cron_expr}</span>
      </div>
      {isRunning && (
        <div className="task-progress">
          <div className="progress-bar-bg">
            <div className="progress-bar-fill" style={{ width: `${progress?.percent ?? 0}%` }} />
          </div>
          <span className="progress-text">{progress?.percent ?? 0}% {progress?.speed ?? ''} Kalan: {progress?.eta ?? '-'}</span>
        </div>
      )}
      <div className="task-actions">
        <button onClick={() => onRunNow(task.id)} title="Şimdi Çalıştır">▶ Çalıştır</button>
        <button onClick={() => onToggle(task.id)}>{task.enabled ? "⏸ Duraklat" : "▶ Devam Et"}</button>
        {isRunning && <button onClick={() => onStop(task.id)} className="btn-stop">⏹ Durdur</button>}
        <button onClick={() => onEdit(task)}>Düzenle</button>
        <button onClick={() => onDelete(task.id)}>Sil</button>
      </div>
    </div>
  );
}
