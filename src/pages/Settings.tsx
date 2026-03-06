import React, { useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import { useSettingsStore } from "../store/settingsStore";
import type { AppSettings } from "../types";

export function Settings() {
  const navigate = useNavigate();
  const { settings, setSettings } = useSettingsStore();
  const [local, setLocal] = useState<AppSettings>(settings);
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    invoke<AppSettings>("get_settings")
      .then((s) => {
        setSettings(s);
        setLocal(s);
      })
      .catch(() => {});
  }, []);

  async function handleSave() {
    try {
      await invoke("save_settings", { settings: local });
      setSettings(local);
      setSaved(true);
      setTimeout(() => setSaved(false), 2000);
    } catch (e) {
      alert(`保存失败: ${e}`);
    }
  }

  const inputStyle: React.CSSProperties = {
    width: "100%",
    padding: "8px 12px",
    border: "1px solid #d1d5db",
    borderRadius: 6,
    fontSize: 14,
    boxSizing: "border-box",
    outline: "none",
  };

  const labelStyle: React.CSSProperties = {
    display: "block",
    fontSize: 13,
    fontWeight: 500,
    color: "#374151",
    marginBottom: 6,
  };

  return (
    <div style={{ maxWidth: 600, margin: "0 auto", padding: "32px 20px" }}>
      <button
        onClick={() => navigate("/")}
        style={{ background: "none", border: "none", color: "#6b7280", cursor: "pointer", fontSize: 14, marginBottom: 16, padding: 0 }}
      >
        ← 返回
      </button>
      <h2 style={{ margin: "0 0 28px", fontSize: 22, fontWeight: 700, color: "#1f2937" }}>设置</h2>

      <div style={{ display: "flex", flexDirection: "column", gap: 20 }}>
        <section>
          <h3 style={{ fontSize: 15, fontWeight: 600, color: "#374151", marginBottom: 16, paddingBottom: 8, borderBottom: "1px solid #f3f4f6" }}>
            LLM 配置
          </h3>

          <div style={{ marginBottom: 14 }}>
            <label style={labelStyle}>Provider</label>
            <select
              value={local.llm_provider.type}
              onChange={(e) => setLocal({ ...local, llm_provider: { ...local.llm_provider, type: e.target.value as "ollama" | "openai" } })}
              style={{ ...inputStyle, background: "#fff" }}
            >
              <option value="ollama">Ollama（本地）</option>
              <option value="openai">OpenAI</option>
            </select>
          </div>

          <div style={{ marginBottom: 14 }}>
            <label style={labelStyle}>Base URL</label>
            <input
              type="text"
              value={local.llm_provider.base_url}
              onChange={(e) => setLocal({ ...local, llm_provider: { ...local.llm_provider, base_url: e.target.value } })}
              style={inputStyle}
            />
          </div>

          <div style={{ marginBottom: 14 }}>
            <label style={labelStyle}>模型</label>
            <input
              type="text"
              value={local.llm_provider.model}
              onChange={(e) => setLocal({ ...local, llm_provider: { ...local.llm_provider, model: e.target.value } })}
              style={inputStyle}
              placeholder="llama3 / gpt-4o"
            />
          </div>

          {local.llm_provider.type === "openai" && (
            <div style={{ marginBottom: 14 }}>
              <label style={labelStyle}>API Key</label>
              <input
                type="password"
                value={local.llm_provider.api_key || ""}
                onChange={(e) => setLocal({ ...local, llm_provider: { ...local.llm_provider, api_key: e.target.value || null } })}
                style={inputStyle}
                placeholder="sk-..."
              />
            </div>
          )}
        </section>

        <section>
          <h3 style={{ fontSize: 15, fontWeight: 600, color: "#374151", marginBottom: 16, paddingBottom: 8, borderBottom: "1px solid #f3f4f6" }}>
            ASR 配置
          </h3>

          <div style={{ marginBottom: 14 }}>
            <label style={labelStyle}>Whisper 模型</label>
            <select
              value={local.whisper_model}
              onChange={(e) => setLocal({ ...local, whisper_model: e.target.value })}
              style={{ ...inputStyle, background: "#fff" }}
            >
              <option value="tiny">tiny（最快）</option>
              <option value="base">base（推荐）</option>
              <option value="small">small</option>
              <option value="medium">medium</option>
              <option value="large">large（最准）</option>
            </select>
          </div>

          <div style={{ marginBottom: 14 }}>
            <label style={labelStyle}>识别语言</label>
            <select
              value={local.language}
              onChange={(e) => setLocal({ ...local, language: e.target.value })}
              style={{ ...inputStyle, background: "#fff" }}
            >
              <option value="zh">中文</option>
              <option value="en">English</option>
              <option value="auto">自动检测</option>
            </select>
          </div>
        </section>

        <button
          onClick={handleSave}
          style={{
            padding: "12px 24px",
            background: saved ? "#10b981" : "#3b82f6",
            color: "#fff",
            border: "none",
            borderRadius: 8,
            cursor: "pointer",
            fontSize: 15,
            fontWeight: 500,
            transition: "background 0.2s",
          }}
        >
          {saved ? "✓ 已保存" : "保存设置"}
        </button>
      </div>
    </div>
  );
}
