import { create } from "zustand";
import { invoke } from "@tauri-apps/api/core";

interface Memory {
  id: string;
  content: string;
  similarity: number;
  memory_type: string;
}

interface ThinkResult {
  response: string;
  confidence: number;
  thought_id: string;
  memory_count: number;
  ai_enhanced: boolean;
}

interface FileResult {
  path: string;
  name: string;
  chunk: string;
  similarity: number;
  file_type: string;
}

interface WorkflowResult {
  action: string;
  success: boolean;
  message: string;
  data: unknown | null;
}

interface SystemStatus {
  status: string;
  memory_count: number;
  thought_count: number;
  uptime_ms: number;
  ai_provider: string;
  ai_available: boolean;
  embedding_provider: string;
  learning_trend: string;
  indexed_files: number;
  indexed_chunks: number;
}

interface Settings {
  ai_provider: string;
  ollama_model: string;
  claude_api_key: string | null;
  hotkey: string;
  indexed_folders: string[];
  theme: string;
  auto_start: boolean;
  privacy_mode: boolean;
  onboarded: boolean;
}

interface SearchResults {
  memories: Memory[];
  files: FileResult[];
  thinkResult: ThinkResult | null;
}

interface ClipboardEntry {
  content: string;
  timestamp: number;
}

interface AppState {
  query: string;
  results: SearchResults;
  isSearching: boolean;
  recentMemories: Memory[];
  status: SystemStatus | null;
  settings: Settings | null;
  clipboardHistory: ClipboardEntry[];
  mode: "search" | "remember";

  setQuery: (query: string) => void;
  setMode: (mode: "search" | "remember") => void;
  search: (query: string) => Promise<void>;
  searchFiles: (query: string) => Promise<FileResult[]>;
  runWorkflow: (action: string, query?: string) => Promise<WorkflowResult>;
  remember: (content: string, type: string, importance?: number) => Promise<void>;
  think: (input: string) => Promise<ThinkResult>;
  loadStatus: () => Promise<void>;
  loadSettings: () => Promise<void>;
  updateSettings: (settings: Settings) => Promise<void>;
  loadClipboardHistory: () => Promise<void>;
  addIndexedFolder: (path: string) => Promise<void>;
  indexFiles: () => Promise<void>;
  clearResults: () => void;
}

export const useAppStore = create<AppState>((set, get) => ({
  query: "",
  results: { memories: [], files: [], thinkResult: null },
  isSearching: false,
  recentMemories: [],
  status: null,
  settings: null,
  clipboardHistory: [],
  mode: "search",

  setQuery: (query: string) => set({ query }),
  setMode: (mode: "search" | "remember") => set({ mode }),

  search: async (query: string) => {
    set({ query, isSearching: true });

    try {
      // Run recall, think, and file search in parallel
      const [memories, thinkResult, files] = await Promise.all([
        invoke<Memory[]>("recall", { query, limit: 10 }),
        invoke<ThinkResult>("think", { input: query }),
        invoke<FileResult[]>("search_files", { query, limit: 10 }).catch(() => [] as FileResult[]),
      ]);

      set({
        results: { memories, files, thinkResult },
        isSearching: false,
      });
    } catch (error) {
      console.error("Search failed:", error);
      set({ isSearching: false });
    }
  },

  searchFiles: async (query: string) => {
    return invoke<FileResult[]>("search_files", { query, limit: 20 });
  },

  runWorkflow: async (action: string, query?: string) => {
    return invoke<WorkflowResult>("run_workflow", { action, query: query ?? null });
  },

  remember: async (content: string, type: string, importance?: number) => {
    try {
      await invoke("remember", {
        content,
        memoryType: type,
        importance: importance ?? 0.7,
      });
      // Refresh status after storing
      get().loadStatus();
    } catch (error) {
      console.error("Remember failed:", error);
    }
  },

  think: async (input: string) => {
    const result = await invoke<ThinkResult>("think", { input });
    return result;
  },

  loadStatus: async () => {
    try {
      const status = await invoke<SystemStatus>("get_status");
      set({ status });
    } catch (error) {
      console.error("Failed to load status:", error);
    }
  },

  loadSettings: async () => {
    try {
      const settings = await invoke<Settings>("get_settings");
      set({ settings });
    } catch (error) {
      console.error("Failed to load settings:", error);
    }
  },

  updateSettings: async (settings: Settings) => {
    try {
      await invoke("update_settings", { settings });
      set({ settings });
    } catch (error) {
      console.error("Failed to update settings:", error);
    }
  },

  loadClipboardHistory: async () => {
    try {
      const history = await invoke<ClipboardEntry[]>("get_clipboard_history", { limit: 20 });
      set({ clipboardHistory: history });
    } catch (error) {
      console.error("Failed to load clipboard history:", error);
    }
  },

  addIndexedFolder: async (path: string) => {
    try {
      await invoke("add_indexed_folder", { path });
      get().loadSettings();
      get().loadStatus();
    } catch (error) {
      console.error("Failed to add folder:", error);
    }
  },

  indexFiles: async () => {
    try {
      await invoke("index_files");
      get().loadStatus();
    } catch (error) {
      console.error("Failed to index files:", error);
    }
  },

  clearResults: () => {
    set({ query: "", results: { memories: [], files: [], thinkResult: null } });
  },
}));
