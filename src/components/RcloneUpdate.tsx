import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { message } from "@tauri-apps/plugin-dialog";

interface SelfUpdateResult {
  success: boolean;
  old_version: string;
  new_version: string;
  message: string;
}

export default function RcloneUpdate() {
  const [loading, setLoading] = useState(false);

  const handleUpdate = async () => {
    setLoading(true);
    try {
      const result = await invoke<SelfUpdateResult>("rclone_selfupdate");
      if (result.success) {
        await message(
          `${result.old_version} → ${result.new_version}\n\n${result.message}`,
          { title: "✅ rclone Güncellendi", kind: "info" },
        );
      } else {
        await message(result.message, {
          title: "❌ Güncelleme Başarısız",
          kind: "error",
        });
      }
    } catch (e) {
      await message(`Hata: ${e}`, {
        title: "❌ Hata",
        kind: "error",
      });
    } finally {
      setLoading(false);
    }
  };

  return (
    <button
      className="btn-update"
      onClick={handleUpdate}
      disabled={loading}
      title="rclone binary'sini güncelle"
    >
      {loading ? "⏳" : "🔄"} {loading ? "Güncelleniyor..." : "Rclone'u Güncelle"}
    </button>
  );
}
