import { useState, useEffect } from "react";
import { Link } from "react-router-dom";
import { SandboxConfig, listModels, updateSettings, runBenchmark } from "../lib/tauri";

export default function Settings() {
  const [config, setConfig] = useState<SandboxConfig>({
    platform_url: "https://market.workingclaw.com",
    api_token: "",
    ollama_url: "http://localhost:11434",
    model_name: "llama3.1:8b",
    max_concurrent: 5,
    docker_image: "workingclaw/sandbox:latest",
    container_memory_mb: 512,
    container_cpu_limit: 1.0,
    task_timeout_secs: 300,
    poll_interval_secs: 5,
  });
  const [models, setModels] = useState<string[]>([]);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);
  const [benchmarking, setBenchmarking] = useState("");
  const [benchmarkResult, setBenchmarkResult] = useState<string | null>(null);

  useEffect(() => {
    listModels()
      .then(setModels)
      .catch(() => setModels([]));
  }, []);

  async function handleSave() {
    setSaving(true);
    try {
      await updateSettings(config);
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
    } catch (e) {
      alert(String(e));
    } finally {
      setSaving(false);
    }
  }

  async function handleBenchmark(category: string) {
    setBenchmarking(category);
    setBenchmarkResult(null);
    try {
      const score = await runBenchmark(category);
      setBenchmarkResult(
        `${category}: ${score.toFixed(1)}% ${score >= 80 ? "(PASSED)" : "(FAILED — need 80%+)"}`
      );
    } catch (e) {
      setBenchmarkResult(`Error: ${String(e)}`);
    } finally {
      setBenchmarking("");
    }
  }

  function updateField<K extends keyof SandboxConfig>(key: K, value: SandboxConfig[K]) {
    setConfig((prev) => ({ ...prev, [key]: value }));
  }

  return (
    <div className="h-screen overflow-y-auto bg-[#0a0a0f] text-gray-200">
      <header className="flex items-center justify-between px-6 py-4 border-b border-gray-800 sticky top-0 bg-[#0a0a0f] z-10">
        <h1 className="text-lg font-bold">Settings</h1>
        <div className="flex gap-3">
          <Link to="/" className="text-sm text-gray-400 hover:text-white">
            Back to Dashboard
          </Link>
          <button
            onClick={handleSave}
            disabled={saving}
            className="bg-blue-600 text-white px-4 py-2 rounded-lg text-sm font-medium hover:bg-blue-700 disabled:opacity-50"
          >
            {saving ? "Saving..." : saved ? "Saved!" : "Save Settings"}
          </button>
        </div>
      </header>

      <main className="max-w-2xl mx-auto p-6 space-y-8">
        {/* Connection */}
        <section>
          <h2 className="text-sm font-semibold text-gray-400 uppercase tracking-wider mb-4">
            Platform Connection
          </h2>
          <div className="space-y-4">
            <div>
              <label className="block text-sm text-gray-400 mb-1">Platform URL</label>
              <input
                type="url"
                value={config.platform_url}
                onChange={(e) => updateField("platform_url", e.target.value)}
                className="w-full bg-gray-900 border border-gray-700 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-blue-500"
              />
            </div>
            <div>
              <label className="block text-sm text-gray-400 mb-1">API Token</label>
              <input
                type="password"
                value={config.api_token}
                onChange={(e) => updateField("api_token", e.target.value)}
                placeholder="Paste your operator token from the platform"
                className="w-full bg-gray-900 border border-gray-700 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-blue-500"
              />
            </div>
          </div>
        </section>

        {/* Model */}
        <section>
          <h2 className="text-sm font-semibold text-gray-400 uppercase tracking-wider mb-4">
            AI Model
          </h2>
          <div className="space-y-4">
            <div>
              <label className="block text-sm text-gray-400 mb-1">Ollama URL</label>
              <input
                type="url"
                value={config.ollama_url}
                onChange={(e) => updateField("ollama_url", e.target.value)}
                className="w-full bg-gray-900 border border-gray-700 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-blue-500"
              />
            </div>
            <div>
              <label className="block text-sm text-gray-400 mb-1">Model</label>
              {models.length > 0 ? (
                <select
                  value={config.model_name}
                  onChange={(e) => updateField("model_name", e.target.value)}
                  className="w-full bg-gray-900 border border-gray-700 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-blue-500"
                >
                  {models.map((m) => (
                    <option key={m} value={m}>{m}</option>
                  ))}
                </select>
              ) : (
                <input
                  type="text"
                  value={config.model_name}
                  onChange={(e) => updateField("model_name", e.target.value)}
                  placeholder="e.g., llama3.1:8b"
                  className="w-full bg-gray-900 border border-gray-700 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-blue-500"
                />
              )}
              <p className="text-xs text-gray-600 mt-1">
                Minimum: 7B for summaries, 13B+ for email/content
              </p>
            </div>
          </div>
        </section>

        {/* Resources */}
        <section>
          <h2 className="text-sm font-semibold text-gray-400 uppercase tracking-wider mb-4">
            Resources
          </h2>
          <div className="grid grid-cols-2 gap-4">
            <div>
              <label className="block text-sm text-gray-400 mb-1">
                Max Concurrent Tasks
              </label>
              <input
                type="number"
                min={1}
                max={15}
                value={config.max_concurrent}
                onChange={(e) => updateField("max_concurrent", parseInt(e.target.value) || 5)}
                className="w-full bg-gray-900 border border-gray-700 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-blue-500"
              />
              <p className="text-xs text-gray-600 mt-1">Realistic: 5-15 for consumer hardware</p>
            </div>
            <div>
              <label className="block text-sm text-gray-400 mb-1">
                Container Memory (MB)
              </label>
              <input
                type="number"
                min={256}
                max={8192}
                step={256}
                value={config.container_memory_mb}
                onChange={(e) => updateField("container_memory_mb", parseInt(e.target.value) || 512)}
                className="w-full bg-gray-900 border border-gray-700 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-blue-500"
              />
            </div>
            <div>
              <label className="block text-sm text-gray-400 mb-1">
                CPU Limit (cores)
              </label>
              <input
                type="number"
                min={0.5}
                max={16}
                step={0.5}
                value={config.container_cpu_limit}
                onChange={(e) => updateField("container_cpu_limit", parseFloat(e.target.value) || 1)}
                className="w-full bg-gray-900 border border-gray-700 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-blue-500"
              />
            </div>
            <div>
              <label className="block text-sm text-gray-400 mb-1">
                Task Timeout (seconds)
              </label>
              <input
                type="number"
                min={30}
                max={1800}
                value={config.task_timeout_secs}
                onChange={(e) => updateField("task_timeout_secs", parseInt(e.target.value) || 300)}
                className="w-full bg-gray-900 border border-gray-700 rounded-lg px-3 py-2 text-sm focus:outline-none focus:border-blue-500"
              />
            </div>
          </div>
        </section>

        {/* Certification */}
        <section>
          <h2 className="text-sm font-semibold text-gray-400 uppercase tracking-wider mb-4">
            Certification Benchmarks
          </h2>
          <p className="text-xs text-gray-500 mb-4">
            Run benchmarks to activate task categories. Need 80%+ to pass.
          </p>
          <div className="space-y-2">
            {["document-summary", "email-draft", "content-writing"].map((cat) => (
              <div
                key={cat}
                className="flex items-center justify-between bg-gray-900 border border-gray-800 rounded-lg px-4 py-3"
              >
                <span className="text-sm capitalize">{cat.replace("-", " ")}</span>
                <button
                  onClick={() => handleBenchmark(cat)}
                  disabled={!!benchmarking}
                  className="text-xs bg-gray-800 hover:bg-gray-700 px-3 py-1.5 rounded text-gray-300 disabled:opacity-50"
                >
                  {benchmarking === cat ? "Running..." : "Run Benchmark"}
                </button>
              </div>
            ))}
          </div>
          {benchmarkResult && (
            <p className="text-sm mt-3 text-gray-300">{benchmarkResult}</p>
          )}
        </section>
      </main>
    </div>
  );
}
