import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";

window.addEventListener("error", (event) => {
  console.error("Global JS error:", event.error || event.message);
  const root = document.getElementById("root");
  if (root && root.children.length === 0) {
    root.innerHTML = `<div style="padding:20px;color:#d32f2f;background:#ffebee;font-family:sans-serif;border-radius:8px;margin:20px;">
      <h3 style="margin-top:0;">Uygulama Çalıştırma Hatası</h3>
      <p style="font-size:0.9rem;">${event.message}</p>
      <pre style="background:#fff;padding:10px;border-radius:4px;font-size:0.8rem;overflow:auto;">${event.filename}:${event.lineno}:${event.colno}</pre>
    </div>`;
  }
});

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
