import { Task } from "../types";

interface Props {
  task: Task;
  onToggle: (id: string) => void;
  onDelete: (id: string) => void;
  onRunNow: (id: string) => void;
  onEdit: (task: Task) => void;
  isRunning?: boolean;
  progress?: { percent: number; speed: string; eta: string };
}

export default function TaskCard({ task, onToggle, onDelete, onRunNow, onEdit, isRunning, progress }: Props) {
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
      {isRunning && (
        <div className="task-progress">
          <div className="progress-bar-bg">
            <div className="progress-bar-fill" style={{ width: `${progress?.percent ?? 0}%` }} />
          </div>
          <span className="progress-text">{progress?.percent ?? 0}% {progress?.speed ?? ''} ETA: {progress?.eta ?? '-'}</span>
        </div>
      )}
      <div className="task-actions">
        <button onClick={() => onRunNow(task.id)} title="Run Now">▶</button>
        <button onClick={() => onToggle(task.id)}>{task.enabled ? "⏸ Pause" : "▶ Resume"}</button>
        <button onClick={() => onEdit(task)}>Edit</button>
        <button onClick={() => onDelete(task.id)}>Delete</button>
      </div>
    </div>
  );
}
