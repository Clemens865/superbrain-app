import { useCallback, useState } from "react";
import { useAppStore } from "../store/appStore";

interface QuickActionsProps {
  onSettings: () => void;
}

export default function QuickActions({ onSettings }: QuickActionsProps) {
  const { remember, loadStatus, runWorkflow } = useAppStore();
  const [rememberText, setRememberText] = useState("");
  const [showRemember, setShowRemember] = useState(false);

  const handleRememberClipboard = useCallback(async () => {
    try {
      const text = await navigator.clipboard.readText();
      if (text.trim()) {
        await remember(text.trim(), "working", 0.6);
        loadStatus();
      }
    } catch {
      // Clipboard permission denied - show manual input
      setShowRemember(true);
    }
  }, [remember, loadStatus]);

  const handleRememberSubmit = useCallback(async () => {
    if (rememberText.trim()) {
      await remember(rememberText.trim(), "semantic", 0.7);
      setRememberText("");
      setShowRemember(false);
      loadStatus();
    }
  }, [rememberText, remember, loadStatus]);

  if (showRemember) {
    return (
      <div className="px-4 py-3 border-t border-brain-border">
        <div className="flex gap-2">
          <input
            type="text"
            value={rememberText}
            onChange={(e) => setRememberText(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && handleRememberSubmit()}
            placeholder="Type something to remember..."
            className="flex-1 bg-brain-surface text-white text-sm px-3 py-2 rounded-lg border border-brain-border outline-none focus:border-brain-accent/50"
            autoFocus
          />
          <button
            onClick={handleRememberSubmit}
            className="px-3 py-2 bg-brain-accent text-white text-sm rounded-lg hover:bg-brain-accent/80 transition-colors"
          >
            Save
          </button>
          <button
            onClick={() => setShowRemember(false)}
            className="px-3 py-2 text-brain-text/50 text-sm rounded-lg hover:bg-brain-surface transition-colors"
          >
            Cancel
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="px-4 py-3 border-t border-brain-border">
      <div className="flex gap-2">
        <ActionButton
          icon={
            <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4v16m8-8H4" />
            </svg>
          }
          label="Remember"
          onClick={() => setShowRemember(true)}
        />
        <ActionButton
          icon={
            <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 5H7a2 2 0 00-2 2v12a2 2 0 002 2h10a2 2 0 002-2V7a2 2 0 00-2-2h-2M9 5a2 2 0 002 2h2a2 2 0 002-2M9 5a2 2 0 012-2h2a2 2 0 012 2" />
            </svg>
          }
          label="Clipboard"
          onClick={handleRememberClipboard}
        />
        <ActionButton
          icon={
            <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 10V3L4 14h7v7l9-11h-7z" />
            </svg>
          }
          label="Digest"
          onClick={() => runWorkflow("digest")}
        />
        <ActionButton
          icon={
            <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z" />
            </svg>
          }
          label="Status"
          onClick={() => loadStatus()}
        />
        <ActionButton
          icon={
            <svg className="w-3.5 h-3.5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M10.325 4.317c.426-1.756 2.924-1.756 3.35 0a1.724 1.724 0 002.573 1.066c1.543-.94 3.31.826 2.37 2.37a1.724 1.724 0 001.066 2.573c1.756.426 1.756 2.924 0 3.35a1.724 1.724 0 00-1.066 2.573c.94 1.543-.826 3.31-2.37 2.37a1.724 1.724 0 00-2.573 1.066c-.426 1.756-2.924 1.756-3.35 0a1.724 1.724 0 00-2.573-1.066c-1.543.94-3.31-.826-2.37-2.37a1.724 1.724 0 00-1.066-2.573c-1.756-.426-1.756-2.924 0-3.35a1.724 1.724 0 001.066-2.573c-.94-1.543.826-3.31 2.37-2.37.996.608 2.296.07 2.572-1.065z" />
              <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
            </svg>
          }
          label="Settings"
          onClick={onSettings}
        />
      </div>
    </div>
  );
}

function ActionButton({
  icon,
  label,
  onClick,
}: {
  icon: React.ReactNode;
  label: string;
  onClick: () => void;
}) {
  return (
    <button
      onClick={onClick}
      className="flex items-center gap-1.5 px-3 py-2 bg-brain-surface text-brain-text text-xs rounded-lg border border-brain-border hover:border-brain-accent/30 hover:text-white transition-colors"
    >
      {icon}
      {label}
    </button>
  );
}
