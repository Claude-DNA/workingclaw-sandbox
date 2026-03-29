import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export interface SandboxStatus {
  is_online: boolean;
  current_load: number;
  max_concurrent: number;
  docker_available: boolean;
  ollama_available: boolean;
  model_loaded: string;
  tasks_completed: number;
  uptime_seconds: number;
}

export interface SandboxConfig {
  platform_url: string;
  api_token: string;
  ollama_url: string;
  model_name: string;
  max_concurrent: number;
  docker_image: string;
  container_memory_mb: number;
  container_cpu_limit: number;
  task_timeout_secs: number;
  poll_interval_secs: number;
}

export const getStatus = () => invoke<SandboxStatus>("get_status");
export const goOnline = () => invoke<void>("go_online");
export const goOffline = () => invoke<void>("go_offline");
export const updateSettings = (settings: SandboxConfig) =>
  invoke<void>("update_settings", { settings });
export const listModels = () => invoke<string[]>("list_models");
export const runBenchmark = (category: string) =>
  invoke<number>("run_benchmark", { category });

// Event listeners
export const onTaskStarted = (cb: (taskId: string) => void) =>
  listen<string>("task-started", (e) => cb(e.payload));
export const onTaskCompleted = (cb: (data: { task_id: string; duration: number }) => void) =>
  listen<{ task_id: string; duration: number }>("task-completed", (e) => cb(e.payload));
export const onTaskFailed = (cb: (data: { task_id: string; error: string }) => void) =>
  listen<{ task_id: string; error: string }>("task-failed", (e) => cb(e.payload));
export const onPollError = (cb: (error: string) => void) =>
  listen<string>("poll-error", (e) => cb(e.payload));
