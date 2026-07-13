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
