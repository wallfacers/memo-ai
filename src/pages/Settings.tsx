import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";
import { useSettingsStore } from "@/store/settingsStore";
import type { AppSettings } from "@/types";
import { Check } from "lucide-react";

export function Settings() {
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
  }, [setSettings]);

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

  return (
    <div className="max-w-xl mx-auto px-6 py-8 space-y-6">
      <h2 className="text-xl font-semibold text-foreground">设置</h2>

      <Card>
        <CardHeader className="pb-3">
          <CardTitle className="text-xs font-semibold text-muted-foreground uppercase tracking-wide">
            LLM 配置
          </CardTitle>
        </CardHeader>
        <Separator />
        <CardContent className="pt-4 space-y-4">
          <div className="space-y-1.5">
            <label className="text-sm font-medium text-foreground">Provider</label>
            <Select
              value={local.llm_provider.type}
              onValueChange={(v) =>
                setLocal({
                  ...local,
                  llm_provider: { ...local.llm_provider, type: v as "ollama" | "openai" },
                })
              }
            >
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="ollama">Ollama（本地）</SelectItem>
                <SelectItem value="openai">OpenAI</SelectItem>
              </SelectContent>
            </Select>
          </div>

          <div className="space-y-1.5">
            <label className="text-sm font-medium text-foreground">Base URL</label>
            <Input
              value={local.llm_provider.base_url}
              onChange={(e) =>
                setLocal({ ...local, llm_provider: { ...local.llm_provider, base_url: e.target.value } })
              }
            />
          </div>

          <div className="space-y-1.5">
            <label className="text-sm font-medium text-foreground">模型</label>
            <Input
              value={local.llm_provider.model}
              onChange={(e) =>
                setLocal({ ...local, llm_provider: { ...local.llm_provider, model: e.target.value } })
              }
              placeholder="llama3 / gpt-4o"
            />
          </div>

          {local.llm_provider.type === "openai" && (
            <div className="space-y-1.5">
              <label className="text-sm font-medium text-foreground">API Key</label>
              <Input
                type="password"
                value={local.llm_provider.api_key || ""}
                onChange={(e) =>
                  setLocal({
                    ...local,
                    llm_provider: { ...local.llm_provider, api_key: e.target.value || null },
                  })
                }
                placeholder="sk-..."
              />
            </div>
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader className="pb-3">
          <CardTitle className="text-xs font-semibold text-muted-foreground uppercase tracking-wide">
            ASR 配置
          </CardTitle>
        </CardHeader>
        <Separator />
        <CardContent className="pt-4 space-y-4">
          <div className="space-y-1.5">
            <label className="text-sm font-medium text-foreground">Whisper 模型</label>
            <Select
              value={local.whisper_model}
              onValueChange={(v) => setLocal({ ...local, whisper_model: v })}
            >
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="tiny">tiny（最快）</SelectItem>
                <SelectItem value="base">base（推荐）</SelectItem>
                <SelectItem value="small">small</SelectItem>
                <SelectItem value="medium">medium</SelectItem>
                <SelectItem value="large">large（最准）</SelectItem>
              </SelectContent>
            </Select>
          </div>

          <div className="space-y-1.5">
            <label className="text-sm font-medium text-foreground">识别语言</label>
            <Select
              value={local.language}
              onValueChange={(v) => setLocal({ ...local, language: v })}
            >
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="zh">中文</SelectItem>
                <SelectItem value="en">English</SelectItem>
                <SelectItem value="auto">自动检测</SelectItem>
              </SelectContent>
            </Select>
          </div>
        </CardContent>
      </Card>

      <Button onClick={handleSave} className="w-full" size="lg">
        {saved ? (
          <>
            <Check className="mr-2 h-4 w-4" />
            已保存
          </>
        ) : (
          "保存设置"
        )}
      </Button>
    </div>
  );
}
