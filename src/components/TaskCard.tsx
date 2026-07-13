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
