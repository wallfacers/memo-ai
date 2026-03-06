import { create } from "zustand";
import type { AppSettings, AsrProviderType } from "../types";

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
  whisper_model_dir: "models",
  asr_provider: "local_whisper" as AsrProviderType,
  aliyun_asr_app_key: "",
  aliyun_asr_access_key_id: "",
  aliyun_asr_access_key_secret: "",
};

export const useSettingsStore = create<SettingsStore>((set) => ({
  settings: defaultSettings,
  setSettings: (settings) => set({ settings }),
}));
