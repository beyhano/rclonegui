import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import TaskCard from "./TaskCard";
import TaskFormModal from "./TaskFormModal";
import { Task } from "../types";

export default function SchedulerPage() {
  const [tasks, setTasks] = useState<Task[]>([]);
  const [showForm, setShowForm] = useState(false);
  const [editTask, setEditTask] = useState<Task | undefined>(undefined);
  const [loading, setLoading] = useState(true);
  const [runningTasks, setRunningTasks] = useState<Set<string>>(new Set());
  const [taskProgress, setTaskProgress] = useState<Record<string, { percent: number; speed: string; eta: string }>>({});

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
    const unlistenProgress = listen("rclone:progress", (event: any) => {
      const payload = event.payload;
      setTaskProgress(prev => ({ ...prev, [payload.process_id]: { percent: payload.percent, speed: payload.speed, eta: payload.eta } }));
    });
    const unlistenStarted = listen("rclone:process-started", (event: any) => {
      console.log("process started:", event.payload);
    });
    const unlistenCompleted = listen("rclone:process-completed", (event: any) => {
      setTaskProgress(prev => {
        const next = { ...prev };
        delete next[(event.payload as any).process_id];
        return next;
      });
    });
    return () => {
      unlisten1.then(f => f());
      unlisten2.then(f => f());
      unlistenProgress.then(f => f());
      unlistenStarted.then(f => f());
      unlistenCompleted.then(f => f());
    };
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
    setRunningTasks(prev => new Set(prev).add(id));
    try {
      await invoke("task_run_now", { id });
    } finally {
      loadTasks();
      setRunningTasks(prev => { const n = new Set(prev); n.delete(id); return n; });
    }
  };

  const handleStop = async (id: string) => {
    await invoke("task_stop", { id });
    loadTasks();
    setRunningTasks(prev => { const n = new Set(prev); n.delete(id); return n; });
  };

  if (loading) return <div className="scheduler-page"><p>Loading tasks...</p></div>;

  return (
    <div className="scheduler-page">
      <div className="scheduler-header">
        <h2>Scheduled Tasks</h2>
        <button onClick={() => { setEditTask(undefined); setShowForm(true); }}>+ New Task</button>
      </div>
      {tasks.length === 0 ? (
        <div className="empty-state">
          <p>No tasks defined yet.</p>
          <p>Click "New Task" to create your first scheduled operation.</p>
        </div>
      ) : (
        <div className="task-list">
          {tasks.map(task => (
            <TaskCard key={task.id} task={task} onToggle={handleToggle} onDelete={handleDelete} onRunNow={handleRunNow} onEdit={(t) => { setEditTask(t); setShowForm(true); }} onStop={handleStop} isRunning={runningTasks.has(task.id)} progress={runningTasks.has(task.id) ? Object.values(taskProgress)[0] : undefined} />
          ))}
        </div>
      )}
      {showForm && (
        <TaskFormModal
          onClose={() => { setShowForm(false); setEditTask(undefined); }}
          onCreated={() => loadTasks()}
          editTask={editTask}
        />
      )}
    </div>
  );
}
