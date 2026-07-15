import { useState } from "react";
import ConfigPanel from "./ConfigPanel";
import SchedulerPage from "./components/SchedulerPage";
import RcloneUpdate from "./components/RcloneUpdate";
import "./App.css";

type Tab = "config" | "scheduler";

function App() {
  const [activeTab, setActiveTab] = useState<Tab>("config");

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
