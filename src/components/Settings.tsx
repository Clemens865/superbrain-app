import { useEffect, useState, useCallback } from "react";
import { useAppStore } from "../store/appStore";

interface SettingsProps {
  onBack: () => void;
}

export default function Settings({ onBack }: SettingsProps) {
  const { settings, loadSettings, updateSettings, status, addIndexedFolder } = useAppStore();
  const [localSettings, setLocalSettings] = useState(settings);
  const [newFolder, setNewFolder] = useState("");

  useEffect(() => {
    loadSettings();
  }, [loadSettings]);

  useEffect(() => {
    if (settings) {
      setLocalSettings(settings);
    }
  }, [settings]);

  const handleSave = useCallback(async () => {
    if (localSettings) {
      await updateSettings(localSettings);
      onBack();
    }
  }, [localSettings, updateSettings, onBack]);

  if (!localSettings) {
    return (
      <div className="p-4 text-brain-text/50 text-sm">Loading settings...</div>
    );
  }

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="flex items-center h-14 px-4 border-b border-brain-border">
        <button
          onClick={onBack}
          className="text-brain-text/50 hover:text-white mr-3 transition-colors"
        >
          <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 19l-7-7 7-7" />
          </svg>
        </button>
        <span className="text-white font-medium">Settings</span>
        <button
          onClick={handleSave}
          className="ml-auto px-3 py-1.5 bg-brain-accent text-white text-xs rounded-lg hover:bg-brain-accent/80 transition-colors"
        >
          Save
        </button>
      </div>

      {/* Settings content */}
      <div className="flex-1 overflow-y-auto p-4 space-y-6">
        {/* AI Provider */}
        <Section title="AI Provider">
          <Select
            value={localSettings.ai_provider}
            onChange={(v) => setLocalSettings({ ...localSettings, ai_provider: v })}
            options={[
              { value: "ollama", label: "Ollama (Local)" },
              { value: "claude", label: "Claude (Cloud)" },
              { value: "none", label: "None (Memory Only)" },
            ]}
          />

          {localSettings.ai_provider === "ollama" && (
            <div className="mt-3">
              <label className="block text-brain-text/50 text-xs mb-1">Model</label>
              <input
                type="text"
                value={localSettings.ollama_model}
                onChange={(e) =>
                  setLocalSettings({ ...localSettings, ollama_model: e.target.value })
                }
                className="w-full bg-brain-bg text-white text-sm px-3 py-2 rounded-lg border border-brain-border outline-none focus:border-brain-accent/50"
              />
            </div>
          )}

          {localSettings.ai_provider === "claude" && (
            <div className="mt-3">
              <label className="block text-brain-text/50 text-xs mb-1">API Key</label>
              <input
                type="password"
                value={localSettings.claude_api_key || ""}
                onChange={(e) =>
                  setLocalSettings({ ...localSettings, claude_api_key: e.target.value || null })
                }
                placeholder="sk-ant-..."
                className="w-full bg-brain-bg text-white text-sm px-3 py-2 rounded-lg border border-brain-border outline-none focus:border-brain-accent/50"
              />
            </div>
          )}
        </Section>

        {/* Hotkey */}
        <Section title="Global Shortcut">
          <div className="text-brain-text text-sm bg-brain-bg px-3 py-2 rounded-lg border border-brain-border">
            {localSettings.hotkey}
          </div>
        </Section>

        {/* Theme */}
        <Section title="Appearance">
          <Select
            value={localSettings.theme}
            onChange={(v) => setLocalSettings({ ...localSettings, theme: v })}
            options={[
              { value: "dark", label: "Dark" },
              { value: "light", label: "Light" },
              { value: "system", label: "System" },
            ]}
          />
        </Section>

        {/* Indexed Folders */}
        <Section title="Indexed Folders">
          <div className="space-y-2 mb-2">
            {(localSettings?.indexed_folders ?? []).length === 0 ? (
              <p className="text-brain-text/40 text-xs">
                Default: ~/Documents, ~/Desktop, ~/Downloads
              </p>
            ) : (
              (localSettings?.indexed_folders ?? []).map((folder, i) => (
                <div key={i} className="flex items-center gap-2">
                  <span className="flex-1 text-brain-text text-xs bg-brain-bg px-3 py-1.5 rounded-lg border border-brain-border truncate">
                    {folder}
                  </span>
                  <button
                    onClick={() => {
                      const updated = localSettings!.indexed_folders.filter((_, idx) => idx !== i);
                      setLocalSettings({ ...localSettings!, indexed_folders: updated });
                    }}
                    className="text-brain-text/30 hover:text-brain-error text-xs transition-colors"
                  >
                    Remove
                  </button>
                </div>
              ))
            )}
          </div>
          <div className="flex gap-2">
            <input
              type="text"
              value={newFolder}
              onChange={(e) => setNewFolder(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter" && newFolder.trim()) {
                  addIndexedFolder(newFolder.trim());
                  setLocalSettings({
                    ...localSettings!,
                    indexed_folders: [...(localSettings?.indexed_folders ?? []), newFolder.trim()],
                  });
                  setNewFolder("");
                }
              }}
              placeholder="Paste folder path..."
              className="flex-1 bg-brain-bg text-white text-xs px-3 py-1.5 rounded-lg border border-brain-border outline-none focus:border-brain-accent/50"
            />
            <button
              onClick={() => {
                if (newFolder.trim()) {
                  addIndexedFolder(newFolder.trim());
                  setLocalSettings({
                    ...localSettings!,
                    indexed_folders: [...(localSettings?.indexed_folders ?? []), newFolder.trim()],
                  });
                  setNewFolder("");
                }
              }}
              className="px-3 py-1.5 bg-brain-surface text-brain-text text-xs rounded-lg border border-brain-border hover:border-brain-accent/30 transition-colors"
            >
              Add
            </button>
          </div>
        </Section>

        {/* Auto-Start */}
        <Section title="Startup">
          <Toggle
            label="Start at login"
            checked={localSettings?.auto_start ?? false}
            onChange={(v) => setLocalSettings({ ...localSettings!, auto_start: v })}
          />
        </Section>

        {/* Privacy */}
        <Section title="Privacy">
          <Toggle
            label="Privacy Mode (disable cloud AI)"
            checked={localSettings.privacy_mode}
            onChange={(v) => setLocalSettings({ ...localSettings, privacy_mode: v })}
          />
        </Section>

        {/* Info */}
        <Section title="System Info">
          <div className="text-brain-text/50 text-xs space-y-1">
            <p>Embedding: {status?.embedding_provider || "..."}</p>
            <p>AI: {status?.ai_provider || "..."}</p>
            <p>Memories: {status?.memory_count ?? 0}</p>
            <p>Indexed Files: {status?.indexed_files ?? 0} ({status?.indexed_chunks ?? 0} chunks)</p>
            <p>Version: 0.1.0</p>
          </div>
        </Section>
      </div>
    </div>
  );
}

function Section({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div>
      <h3 className="text-brain-text/70 text-xs font-medium uppercase tracking-wider mb-2">
        {title}
      </h3>
      {children}
    </div>
  );
}

function Select({
  value,
  onChange,
  options,
}: {
  value: string;
  onChange: (v: string) => void;
  options: { value: string; label: string }[];
}) {
  return (
    <select
      value={value}
      onChange={(e) => onChange(e.target.value)}
      className="w-full bg-brain-bg text-white text-sm px-3 py-2 rounded-lg border border-brain-border outline-none focus:border-brain-accent/50 appearance-none cursor-pointer"
    >
      {options.map((opt) => (
        <option key={opt.value} value={opt.value}>
          {opt.label}
        </option>
      ))}
    </select>
  );
}

function Toggle({
  label,
  checked,
  onChange,
}: {
  label: string;
  checked: boolean;
  onChange: (v: boolean) => void;
}) {
  return (
    <label className="flex items-center justify-between cursor-pointer">
      <span className="text-brain-text text-sm">{label}</span>
      <div
        onClick={() => onChange(!checked)}
        className={`w-10 h-5 rounded-full transition-colors relative ${
          checked ? "bg-brain-accent" : "bg-brain-border"
        }`}
      >
        <div
          className={`w-4 h-4 rounded-full bg-white absolute top-0.5 transition-transform ${
            checked ? "translate-x-5" : "translate-x-0.5"
          }`}
        />
      </div>
    </label>
  );
}
