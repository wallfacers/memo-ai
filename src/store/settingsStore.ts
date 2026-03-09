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
  whisper_model_dir: "models",
  asr_provider: "local_whisper",
  aliyun_asr_app_key: "",
  aliyun_asr_access_key_id: "",
  aliyun_asr_access_key_secret: "",
  funasr_enabled: false,
  funasr_ws_url: "",
  funasr_server_path: "funasr-server",
  funasr_port: 10095,
  qwen3_asr_url: "http://localhost:8000",
};

export const useSettingsStore = create<SettingsStore>((set) => ({
  settings: defaultSettings,
  setSettings: (settings) => set({ settings }),
}));
