import { create } from "zustand";
import type { AppSettings } from "../types";

interface SettingsStore {
  settings: AppSettings;
  setSettings: (settings: AppSettings) => void;
}

const defaultSettings: AppSettings = {
  llm_provider: {
    type: "ollama",
    base_url: "http://localhost:11434",
    model: "llama3",
    api_key: null,
  },
  whisper_model: "base",
  language: "zh",
  whisper_cli_path: "whisper-cli",
};

export const useSettingsStore = create<SettingsStore>((set) => ({
  settings: defaultSettings,
  setSettings: (settings) => set({ settings }),
}));
