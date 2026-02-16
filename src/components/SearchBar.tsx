import { useRef, useCallback } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { useAppStore } from "../store/appStore";

let debounceTimer: ReturnType<typeof setTimeout>;

export default function SearchBar() {
  const inputRef = useRef<HTMLInputElement>(null);
  const { query, setQuery, search, isSearching, clearResults, mode, setMode, remember } =
    useAppStore();

  const handleChange = useCallback(
    (e: React.ChangeEvent<HTMLInputElement>) => {
      const value = e.target.value;
      setQuery(value);

      if (mode === "remember") return; // Don't auto-search in remember mode

      clearTimeout(debounceTimer);
      if (value.trim().length === 0) {
        clearResults();
        return;
      }

      debounceTimer = setTimeout(() => {
        search(value.trim());
      }, 150);
    },
    [setQuery, search, clearResults, mode],
  );

  const handleKeyDown = useCallback(
    async (e: React.KeyboardEvent) => {
      if (e.key === "Escape") {
        if (query) {
          clearResults();
          setQuery("");
        } else {
          getCurrentWindow().hide();
        }
      } else if (e.key === "Enter" && query.trim()) {
        if (mode === "remember") {
          await remember(query.trim(), "semantic", 0.7);
          setQuery("");
        } else {
          search(query.trim());
        }
      } else if (e.key === "Tab") {
        e.preventDefault();
        setMode(mode === "search" ? "remember" : "search");
      }
    },
    [query, search, clearResults, setQuery, mode, setMode, remember],
  );

  const isRemember = mode === "remember";

  return (
    <div className="flex items-center h-16 px-4 border-b border-brain-border">
      {/* Mode toggle */}
      <button
        onClick={() => setMode(isRemember ? "search" : "remember")}
        className={`mr-3 transition-colors ${
          isRemember ? "text-brain-success" : "text-brain-text/50"
        }`}
        title={`${isRemember ? "Remember" : "Search"} mode (Tab to switch)`}
      >
        {isSearching ? (
          <svg className="w-5 h-5 animate-spin" fill="none" viewBox="0 0 24 24">
            <circle className="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" strokeWidth="4" />
            <path className="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4zm2 5.291A7.962 7.962 0 014 12H0c0 3.042 1.135 5.824 3 7.938l3-2.647z" />
          </svg>
        ) : isRemember ? (
          <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 4v16m8-8H4" />
          </svg>
        ) : (
          <svg className="w-5 h-5" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z" />
          </svg>
        )}
      </button>

      {/* Input */}
      <input
        ref={inputRef}
        data-search-input
        type="text"
        value={query}
        onChange={handleChange}
        onKeyDown={handleKeyDown}
        placeholder={isRemember ? "Type something to remember... (Enter to save, Tab to switch)" : "Search, ask, or remember... (Tab to switch mode)"}
        className={`flex-1 bg-transparent text-lg outline-none ${
          isRemember ? "text-brain-success placeholder-brain-success/30" : "text-white placeholder-brain-text/40"
        }`}
        autoFocus
        spellCheck={false}
      />

      {/* Mode badge */}
      <span className={`text-[10px] px-2 py-0.5 rounded-full ml-2 ${
        isRemember ? "bg-brain-success/20 text-brain-success" : "bg-brain-surface text-brain-text/40"
      }`}>
        {isRemember ? "Remember" : "Search"}
      </span>

      {/* Clear button */}
      {query && (
        <button
          onClick={() => clearResults()}
          className="text-brain-text/40 hover:text-brain-text ml-2 transition-colors"
        >
          <svg className="w-4 h-4" fill="none" stroke="currentColor" viewBox="0 0 24 24">
            <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M6 18L18 6M6 6l12 12" />
          </svg>
        </button>
      )}
    </div>
  );
}
