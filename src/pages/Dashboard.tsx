import { useState, useEffect, useCallback } from "react";
import { Link } from "react-router-dom";
import {
  SandboxStatus,
  getStatus,
  goOnline,
  goOffline,
  onTaskStarted,
  onTaskCompleted,
  onTaskFailed,
  onPollError,
} from "../lib/tauri";

interface TaskEvent {
  id: string;
  type: "started" | "completed" | "failed";
  time: string;
  detail?: string;
}

export default function Dashboard() {
  const [status, setStatus] = useState<SandboxStatus | null>(null);
  const [error, setError] = useState("");
  const [toggling, setToggling] = useState(false);
  const [events, setEvents] = useState<TaskEvent[]>([]);

  const refreshStatus = useCallback(async () => {
    try {
      const s = await getStatus();
      setStatus(s);
    } catch (e) {
      setError(String(e));
    }
  }, []);

  useEffect(() => {
    refreshStatus();
    const interval = setInterval(refreshStatus, 3000);

    // Listen for task events from Rust backend
    const unsubs = [
      onTaskStarted((taskId) => {
        setEvents((prev) => [
          { id: taskId, type: "started", time: new Date().toLocaleTimeString() },
          ...prev.slice(0, 49),
        ]);
      }),
      onTaskCompleted((data) => {
        setEvents((prev) => [
          {
            id: data.task_id,
            type: "completed",
            time: new Date().toLocaleTimeString(),
            detail: `${data.duration}s`,
          },
          ...prev.slice(0, 49),
        ]);
      }),
      onTaskFailed((data) => {
        setEvents((prev) => [
          {
            id: data.task_id,
            type: "failed",
            time: new Date().toLocaleTimeString(),
            detail: data.error,
          },
          ...prev.slice(0, 49),
        ]);
      }),
      onPollError((err) => setError(err)),
    ];

    return () => {
      clearInterval(interval);
      unsubs.forEach((p) => p.then((fn) => fn()));
    };
  }, [refreshStatus]);

  async function handleToggle() {
    setToggling(true);
    setError("");
    try {
      if (status?.is_online) {
        await goOffline();
      } else {
        await goOnline();
      }
      await refreshStatus();
    } catch (e) {
      setError(String(e));
    } finally {
      setToggling(false);
    }
  }

  function formatUptime(secs: number): string {
    const h = Math.floor(secs / 3600);
    const m = Math.floor((secs % 3600) / 60);
    return h > 0 ? `${h}h ${m}m` : `${m}m`;
  }

  if (!status) {
    return (
      <div className="h-screen flex items-center justify-center">
        <p className="text-gray-500">Loading...</p>
      </div>
    );
  }

  return (
    <div className="h-screen flex flex-col bg-[#0a0a0f] text-gray-200">
      {/* Header */}
      <header className="flex items-center justify-between px-6 py-4 border-b border-gray-800">
        <div className="flex items-center gap-3">
          <h1 className="text-lg font-bold">WorkingClaw Sandbox 🦞</h1>
          <span className="text-xs bg-blue-600 px-2 py-0.5 rounded">OPERATOR</span>
        </div>
        <div className="flex items-center gap-4">
          <Link to="/settings" className="text-sm text-gray-400 hover:text-white">
            Settings
          </Link>
          <button
            onClick={handleToggle}
            disabled={toggling}
            className={`px-4 py-2 rounded-lg text-sm font-medium transition-colors ${
              status.is_online
                ? "bg-red-600/20 text-red-400 border border-red-700 hover:bg-red-600/30"
                : "bg-green-600/20 text-green-400 border border-green-700 hover:bg-green-600/30"
            }`}
          >
            {toggling ? "..." : status.is_online ? "Go Offline" : "Go Online"}
          </button>
        </div>
      </header>

      {/* Error banner */}
      {error && (
        <div className="bg-red-900/30 border-b border-red-800 px-6 py-3">
          <p className="text-red-300 text-sm">{error}</p>
        </div>
      )}

      {/* Stats */}
      <div className="grid grid-cols-4 gap-4 p-6">
        <div className="bg-gray-900 border border-gray-800 rounded-xl p-4">
          <p className="text-xs text-gray-500 uppercase">Status</p>
          <div className="flex items-center gap-2 mt-2">
            <span
              className={`w-2 h-2 rounded-full ${
                status.is_online ? "bg-green-500 animate-pulse" : "bg-gray-600"
              }`}
            />
            <span className="font-medium">
              {status.is_online ? "Online" : "Offline"}
            </span>
          </div>
        </div>
        <div className="bg-gray-900 border border-gray-800 rounded-xl p-4">
          <p className="text-xs text-gray-500 uppercase">Load</p>
          <p className="text-2xl font-bold mt-1">
            {status.current_load}
            <span className="text-sm text-gray-500">/{status.max_concurrent}</span>
          </p>
        </div>
        <div className="bg-gray-900 border border-gray-800 rounded-xl p-4">
          <p className="text-xs text-gray-500 uppercase">Tasks Completed</p>
          <p className="text-2xl font-bold mt-1">{status.tasks_completed}</p>
        </div>
        <div className="bg-gray-900 border border-gray-800 rounded-xl p-4">
          <p className="text-xs text-gray-500 uppercase">Uptime</p>
          <p className="text-2xl font-bold mt-1">
            {formatUptime(status.uptime_seconds)}
          </p>
        </div>
      </div>

      {/* System checks */}
      <div className="px-6 pb-4">
        <div className="flex gap-4">
          <div
            className={`flex items-center gap-2 text-xs px-3 py-1.5 rounded border ${
              status.docker_available
                ? "border-green-800 text-green-400 bg-green-900/20"
                : "border-red-800 text-red-400 bg-red-900/20"
            }`}
          >
            <span className={`w-1.5 h-1.5 rounded-full ${status.docker_available ? "bg-green-500" : "bg-red-500"}`} />
            Docker {status.docker_available ? "Ready" : "Not Running"}
          </div>
          <div
            className={`flex items-center gap-2 text-xs px-3 py-1.5 rounded border ${
              status.ollama_available
                ? "border-green-800 text-green-400 bg-green-900/20"
                : "border-red-800 text-red-400 bg-red-900/20"
            }`}
          >
            <span className={`w-1.5 h-1.5 rounded-full ${status.ollama_available ? "bg-green-500" : "bg-red-500"}`} />
            Ollama {status.ollama_available ? "Ready" : "Not Running"}
          </div>
          <div className="flex items-center gap-2 text-xs px-3 py-1.5 rounded border border-gray-700 text-gray-400">
            Model: {status.model_loaded || "None"}
          </div>
        </div>
      </div>

      {/* Task event log */}
      <div className="flex-1 px-6 pb-6 overflow-hidden">
        <h2 className="text-xs text-gray-500 uppercase font-semibold mb-3">
          Activity
        </h2>
        <div className="bg-gray-900 border border-gray-800 rounded-xl overflow-y-auto h-full">
          {events.length === 0 ? (
            <div className="flex items-center justify-center h-full text-gray-600 text-sm">
              {status.is_online
                ? "Waiting for tasks..."
                : "Go online to start receiving tasks"}
            </div>
          ) : (
            <div className="divide-y divide-gray-800/50">
              {events.map((evt, i) => (
                <div key={i} className="px-4 py-2.5 flex items-center gap-3">
                  <span className="text-xs text-gray-600 w-16">{evt.time}</span>
                  <span
                    className={`text-xs px-1.5 py-0.5 rounded ${
                      evt.type === "completed"
                        ? "bg-green-900/30 text-green-400"
                        : evt.type === "failed"
                        ? "bg-red-900/30 text-red-400"
                        : "bg-blue-900/30 text-blue-400"
                    }`}
                  >
                    {evt.type}
                  </span>
                  <span className="text-sm text-gray-300 font-mono truncate">
                    {evt.id.slice(0, 12)}
                  </span>
                  {evt.detail && (
                    <span className="text-xs text-gray-500 ml-auto">{evt.detail}</span>
                  )}
                </div>
              ))}
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
