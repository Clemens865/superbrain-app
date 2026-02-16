import { useEffect, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import SearchBar from "./components/SearchBar";
import ResultsList from "./components/ResultsList";
import QuickActions from "./components/QuickActions";
import MemoryFeed from "./components/MemoryFeed";
import Settings from "./components/Settings";
import { useAppStore } from "./store/appStore";

type View = "search" | "settings";

function App() {
  const [view, setView] = useState<View>("search");
  const [expanded, setExpanded] = useState(false);
  const { query, results, isSearching, loadStatus } = useAppStore();

  useEffect(() => {
    loadStatus();

    // Listen for navigation events from tray
    const unlisten = listen<string>("navigate", (event) => {
      if (event.payload === "settings") {
        setView("settings");
        setExpanded(true);
      }
    });

    // Listen for overlay show/hide
    const unlistenShow = listen("overlay-shown", () => {
      // Focus the search input when overlay is shown
      const input = document.querySelector<HTMLInputElement>("[data-search-input]");
      input?.focus();
    });

    return () => {
      unlisten.then((fn) => fn());
      unlistenShow.then((fn) => fn());
    };
  }, [loadStatus]);

  // Expand when we have results or query
  useEffect(() => {
    if (query.length > 0 || results.memories.length > 0) {
      setExpanded(true);
    }
  }, [query, results]);

  if (view === "settings") {
    return (
      <div className="w-full h-screen bg-brain-bg/95 backdrop-blur-xl rounded-2xl border border-brain-border overflow-hidden animate-fade-in">
        <Settings onBack={() => setView("search")} />
      </div>
    );
  }

  return (
    <div
      className={`w-full bg-brain-bg/95 backdrop-blur-xl rounded-2xl border border-brain-border overflow-hidden transition-all duration-150 animate-slide-down ${
        expanded ? "h-[480px]" : "h-16"
      }`}
    >
      {/* Drag region */}
      <div data-tauri-drag-region className="absolute top-0 left-0 right-0 h-3 cursor-move" />

      {/* Search bar */}
      <SearchBar />

      {expanded && (
        <div className="flex flex-col h-[calc(100%-64px)] animate-fade-in">
          {/* Tab area */}
          {isSearching || results.memories.length > 0 ? (
            <ResultsList />
          ) : (
            <div className="flex-1 flex flex-col">
              <MemoryFeed />
              <QuickActions
                onSettings={() => {
                  setView("settings");
                  setExpanded(true);
                }}
              />
            </div>
          )}
        </div>
      )}
    </div>
  );
}

export default App;
