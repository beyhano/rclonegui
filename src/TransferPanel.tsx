import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import type { ProgressPayload, TransferRecord } from "./types";

function TransferPanel() {
  const [source, setSource] = useState("");
  const [dest, setDest] = useState("");
  const [currentJob, setCurrentJob] = useState<{
    processId: string;
    source: string;
    dest: string;
  } | null>(null);
  const [progress, setProgress] = useState<ProgressPayload | null>(null);
  const [history, setHistory] = useState<TransferRecord[]>([]);
  const [error, setError] = useState<string | null>(null);

  // Store process metadata for event callbacks (stable ref, always up to date)
  const processMeta = useRef<Map<string, { source: string; dest: string }>>(
    new Map(),
  );
  // Track the current process id for progress filtering
  const progressPidRef = useRef<string | null>(null);

  useEffect(() => {
    const unlisteners: Array<() => void> = [];
    let cancelled = false;

    async function setup() {
      const u1 = await listen<ProgressPayload>(
        "rclone:progress",
        (event) => {
          if (cancelled) return;
          if (event.payload.process_id === progressPidRef.current) {
            setProgress(event.payload);
          }
        },
      );
      unlisteners.push(u1);

      const u2 = await listen<{
        process_id: string;
        exit_code: number;
      }>("rclone:process-completed", (event) => {
        if (cancelled) return;
        const { process_id, exit_code } = event.payload;
        const meta = processMeta.current.get(process_id);
        if (meta) {
          processMeta.current.delete(process_id);
          setHistory((prev) => [
            {
              id: process_id,
              remote_src: meta.source,
              remote_dest: meta.dest,
              status: exit_code === 0 ? "completed" : "failed",
              progress: 100,
              speed: null,
              started_at: new Date().toISOString(),
              completed_at: new Date().toISOString(),
              error_message:
                exit_code !== 0 ? `Exit code: ${exit_code}` : null,
            },
            ...prev,
          ]);
        }
        setCurrentJob((curr) =>
          curr?.processId === process_id ? null : curr,
        );
        setProgress((curr) =>
          curr?.process_id === process_id ? null : curr,
        );
      });
      unlisteners.push(u2);

      const u3 = await listen<{
        process_id: string;
        exit_code: number;
        stderr_lines: string[];
      }>("rclone:process-error", (event) => {
        if (cancelled) return;
        const { process_id, exit_code, stderr_lines } = event.payload;
        const meta = processMeta.current.get(process_id);
        if (meta) {
          processMeta.current.delete(process_id);
          const errMsg =
            stderr_lines?.join("; ") || `Exit code: ${exit_code}`;
          setHistory((prev) => [
            {
              id: process_id,
              remote_src: meta.source,
              remote_dest: meta.dest,
              status: "failed",
              progress: 100,
              speed: null,
              started_at: new Date().toISOString(),
              completed_at: new Date().toISOString(),
              error_message: errMsg,
            },
            ...prev,
          ]);
        }
        setCurrentJob((curr) =>
          curr?.processId === process_id ? null : curr,
        );
        setProgress((curr) =>
          curr?.process_id === process_id ? null : curr,
        );
      });
      unlisteners.push(u3);
    }

    setup();

    return () => {
      cancelled = true;
      unlisteners.forEach((u) => u());
    };
  }, []);

  async function startTransfer() {
    if (!source || !dest) return;
    setError(null);
    try {
      const pid = await invoke<string>("rclone_exec", {
        args: ["copy", source, dest],
      });
      processMeta.current.set(pid, { source, dest });
      progressPidRef.current = pid;
      setCurrentJob({ processId: pid, source, dest });
      setProgress(null);
    } catch (err: unknown) {
      setError(String(err));
    }
  }

  async function stopTransfer() {
    if (!currentJob) return;
    setError(null);
    try {
      await invoke("rclone_stop", { process_id: currentJob.processId });
    } catch (err: unknown) {
      setError(String(err));
    }
  }

  const progressPercent = progress?.percent ?? 0;
  const isRunning = currentJob !== null;

  return (
    <div className="transfer-panel">
      <h2>Transfer Files</h2>

      <div className="transfer-inputs">
        <input
          placeholder="Source (e.g., /local/path or remote:path)"
          value={source}
          onChange={(e) => setSource(e.currentTarget.value)}
          disabled={isRunning}
        />
        <input
          placeholder="Destination (e.g., gdrive:backup)"
          value={dest}
          onChange={(e) => setDest(e.currentTarget.value)}
          disabled={isRunning}
        />
        <button
          onClick={startTransfer}
          disabled={isRunning || !source || !dest}
        >
          Start Transfer
        </button>
        {isRunning && (
          <button onClick={stopTransfer} className="btn-danger">
            Stop
          </button>
        )}
      </div>

      {error && <div className="panel-error">{error}</div>}

      {isRunning && progress && (
        <div className="progress-section">
          <div className="progress-bar">
            <div
              className="progress-fill"
              style={{ width: `${progressPercent}%` }}
            />
          </div>
          <div className="progress-info">
            <span>
              {progress.transferred} / {progress.total}
            </span>
            <span>{progress.speed}</span>
            <span>ETA: {progress.eta}</span>
            <span>{Math.round(progressPercent)}%</span>
          </div>
        </div>
      )}

      {history.length > 0 && (
        <div className="history-section">
          <h3>Transfer History</h3>
          <table className="history-table">
            <thead>
              <tr>
                <th>Source</th>
                <th>Destination</th>
                <th>Status</th>
                <th>Error</th>
              </tr>
            </thead>
            <tbody>
              {history.map((t) => (
                <tr key={t.id}>
                  <td>{t.remote_src}</td>
                  <td>{t.remote_dest}</td>
                  <td>
                    <span className={`badge badge-${t.status}`}>
                      {t.status}
                    </span>
                  </td>
                  <td>{t.error_message ?? "-"}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}
    </div>
  );
}

export default TransferPanel;
