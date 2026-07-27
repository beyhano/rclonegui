import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import Swal from "sweetalert2";
import ConfigPanel from "./ConfigPanel";
import SchedulerPage from "./components/SchedulerPage";
import RcloneUpdate from "./components/RcloneUpdate";
import "./App.css";

type Tab = "config" | "scheduler";

interface AppUpdateStatus {
  available: boolean;
  version: string;
  current_version: string;
  body?: string;
}

function App() {
  const [activeTab, setActiveTab] = useState<Tab>("config");

  useEffect(() => {
    async function checkForUpdates() {
      try {
        const update = await invoke<AppUpdateStatus>("check_app_update");
        if (update.available) {
          const res = await Swal.fire({
            title: `🚀 Yeni Sürüm Mevcut! (v${update.version})`,
            html: `
              <p style="font-size:0.95rem;">Mevcut sürüm: <b>v${update.current_version}</b></p>
              ${update.body ? `<pre style="text-align:left;background:#f5f5f5;padding:0.5rem;border-radius:4px;font-size:0.85rem;">${update.body}</pre>` : ""}
              <p>RCloneGUI'yi şimdi güncelleyip yeniden başlatmak ister misiniz?</p>
            `,
            icon: "info",
            showCancelButton: true,
            confirmButtonText: "🔄 Şimdi Güncelle",
            cancelButtonText: "Daha Sonra",
            confirmButtonColor: "#2e7d32",
          });

          if (res.isConfirmed) {
            Swal.fire({
              title: "⏳ Güncelleniyor...",
              text: "Yeni sürüm indiriliyor ve kuruluyor, lütfen bekleyin.",
              allowOutsideClick: false,
              didOpen: () => {
                Swal.showLoading();
              },
            });
            await invoke("install_app_update");
          }
        }
      } catch (err) {
        console.warn("App update check error:", err);
      }
    }

    checkForUpdates();
  }, []);

  return (
    <div className="app">
      <div className="header">
        <h1>rclone GUI</h1>
        <RcloneUpdate />
      </div>
      <nav className="tabs">
        <button
          className={`tab ${activeTab === "config" ? "active" : ""}`}
          onClick={() => setActiveTab("config")}
        >
          Uzak Sunucular
        </button>
        <button
          className={`tab ${activeTab === "scheduler" ? "active" : ""}`}
          onClick={() => setActiveTab("scheduler")}
        >
          Zamanlanmış Görevler
        </button>
      </nav>
      <main className="panel">
        {activeTab === "config" && <ConfigPanel />}
        {activeTab === "scheduler" && <SchedulerPage />}
      </main>
    </div>
  );
}

export default App;
