import { useState } from "react";
import ConfigPanel from "./ConfigPanel";
import TransferPanel from "./TransferPanel";
import MountPanel from "./MountPanel";
import SchedulerPage from "./components/SchedulerPage";
import "./App.css";

type Tab = "config" | "transfer" | "mounts" | "scheduler";

function App() {
  const [activeTab, setActiveTab] = useState<Tab>("config");

  return (
    <div className="app">
      <h1>rclone GUI</h1>
      <nav className="tabs">
        <button
          className={`tab ${activeTab === "config" ? "active" : ""}`}
          onClick={() => setActiveTab("config")}
        >
          Config
        </button>
        <button
          className={`tab ${activeTab === "transfer" ? "active" : ""}`}
          onClick={() => setActiveTab("transfer")}
        >
          Transfer
        </button>
        <button
          className={`tab ${activeTab === "mounts" ? "active" : ""}`}
          onClick={() => setActiveTab("mounts")}
        >
          Mounts
        </button>
        <button
          className={`tab ${activeTab === "scheduler" ? "active" : ""}`}
          onClick={() => setActiveTab("scheduler")}
        >Scheduler</button>
      </nav>
      <main className="panel">
        {activeTab === "config" && <ConfigPanel />}
        {activeTab === "transfer" && <TransferPanel />}
        {activeTab === "mounts" && <MountPanel />}
        {activeTab === "scheduler" && <SchedulerPage />}
      </main>
    </div>
  );
}

export default App;
