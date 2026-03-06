import React, { useEffect, useState } from "react";
import { Button } from "@/components/ui/button";
import { useGetSettings, useSaveSettings } from "@/hooks/useTauriCommands";
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
import type { AsrProviderType } from "@/types";
import { Check, Loader2, CheckCircle2, XCircle, Eye, EyeOff } from "lucide-react";
import { useTestLlmConnection, useCheckWhisperCli, useTestAsrConnection } from "@/hooks/useTauriCommands";
import type { LlmTestResult, WhisperCheckResult, AsrTestResult } from "@/hooks/useTauriCommands";
import { useTranslation } from "react-i18next";
import i18n, { saveLang } from "@/i18n";

export function Settings() {
  const { settings, setSettings } = useSettingsStore();
  const [local, setLocal] = useState<AppSettings>(settings);
  const [saved, setSaved] = useState(false);
  const [saveError, setSaveError] = useState<string | null>(null);
  const savedTimerRef = React.useRef<ReturnType<typeof setTimeout> | null>(null);
  const getSettings = useGetSettings();
  const saveSettings = useSaveSettings();
  const testLlmConnection = useTestLlmConnection();
  type TestStatus = "idle" | "testing" | "ok" | "fail";
  const [llmTestStatus, setLlmTestStatus] = React.useState<TestStatus>("idle");
  const [llmTestResult, setLlmTestResult] = React.useState<LlmTestResult | null>(null);
  const checkWhisperCli = useCheckWhisperCli();
  const testAsrConnection = useTestAsrConnection();
  const [whisperCheck, setWhisperCheck] = React.useState<WhisperCheckResult | null>(null);
  const [whisperChecking, setWhisperChecking] = React.useState(false);
  const [asrTestResult, setAsrTestResult] = React.useState<AsrTestResult | null>(null);
  const [asrTesting, setAsrTesting] = React.useState(false);
  const [showAliyunSecret, setShowAliyunSecret] = React.useState(false);
  const { t } = useTranslation();
  const [currentLang, setCurrentLang] = useState(i18n.language);

  useEffect(() => {
    getSettings()
      .then((s) => {
        setSettings(s);
        setLocal(s);
      })
      .catch(() => {});
  }, [getSettings, setSettings]);

  useEffect(() => {
    return () => {
      if (savedTimerRef.current) clearTimeout(savedTimerRef.current);
    };
  }, []);

  async function handleSave() {
    try {
      await saveSettings(local);
      setSettings(local);
      setSaved(true);
      if (savedTimerRef.current) clearTimeout(savedTimerRef.current);
      savedTimerRef.current = setTimeout(() => setSaved(false), 2000);
    } catch (e) {
      setSaveError(String(e));
    }
  }

  function handleLangChange(lang: string) {
    i18n.changeLanguage(lang);
    saveLang(lang);
    setCurrentLang(lang);
  }

  return (
    <div className="flex-1 overflow-auto">
    <div className="max-w-xl mx-auto px-6 py-8 space-y-6">
      <h2 className="text-xl font-semibold text-foreground">{t("settings.title")}</h2>

      {/* 界面语言 */}
      <Card>
        <CardHeader>
          <CardTitle className="text-xs font-semibold text-muted-foreground uppercase tracking-wide">
            {t("settings.language.sectionTitle")}
          </CardTitle>
        </CardHeader>
        <Separator />
        <CardContent className="space-y-4">
          <div className="flex gap-2">
            {(["zh", "en"] as const).map((lang) => (
              <button
                key={lang}
                onClick={() => handleLangChange(lang)}
                className={`px-4 py-1.5 rounded-md text-sm font-medium border transition-colors ${
                  currentLang === lang
                    ? "bg-primary text-primary-foreground border-primary"
                    : "bg-background text-foreground border-input hover:bg-accent"
                }`}
              >
                {t(`settings.language.${lang}`)}
              </button>
            ))}
          </div>
        </CardContent>
      </Card>

      {/* LLM 配置 */}
      <Card>
        <CardHeader>
          <CardTitle className="text-xs font-semibold text-muted-foreground uppercase tracking-wide">
            {t("settings.llm.sectionTitle")}
          </CardTitle>
        </CardHeader>
        <Separator />
        <CardContent className="space-y-4">
          <div className="space-y-1.5">
            <label className="text-sm font-medium text-foreground">{t("settings.llm.provider")}</label>
            <Select
              value={local.llm_provider.type}
              onValueChange={(v) => {
                setLocal({
                  ...local,
                  llm_provider: { ...local.llm_provider, type: v as "ollama" | "openai" },
                });
                setLlmTestStatus("idle");
                setLlmTestResult(null);
              }}
            >
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="ollama">{t("settings.llm.providerOllama")}</SelectItem>
                <SelectItem value="openai">{t("settings.llm.providerOpenAI")}</SelectItem>
              </SelectContent>
            </Select>
          </div>

          <div className="space-y-1.5">
            <label className="text-sm font-medium text-foreground">{t("settings.llm.baseUrl")}</label>
            <Input
              value={local.llm_provider.base_url}
              onChange={(e) =>
                setLocal({ ...local, llm_provider: { ...local.llm_provider, base_url: e.target.value } })
              }
            />
          </div>

          <div className="space-y-1.5">
            <label className="text-sm font-medium text-foreground">{t("settings.llm.model")}</label>
            <Input
              value={local.llm_provider.model}
              onChange={(e) =>
                setLocal({ ...local, llm_provider: { ...local.llm_provider, model: e.target.value } })
              }
              placeholder={t("settings.llm.modelPlaceholder")}
            />
          </div>

          {local.llm_provider.type === "openai" && (
            <div className="space-y-1.5">
              <label className="text-sm font-medium text-foreground">{t("settings.llm.apiKey")}</label>
              <Input
                type="password"
                value={local.llm_provider.api_key || ""}
                onChange={(e) =>
                  setLocal({
                    ...local,
                    llm_provider: { ...local.llm_provider, api_key: e.target.value || null },
                  })
                }
                placeholder={t("settings.llm.apiKeyPlaceholder")}
              />
            </div>
          )}
          <div className="flex items-center gap-3 pt-1">
            <Button
              variant="outline"
              size="sm"
              disabled={llmTestStatus === "testing"}
              onClick={async () => {
                setLlmTestStatus("testing");
                setLlmTestResult(null);
                try {
                  const result = await testLlmConnection(local);
                  setLlmTestResult(result);
                  setLlmTestStatus(result.success ? "ok" : "fail");
                } catch (e) {
                  setLlmTestResult({ success: false, message: String(e), latency_ms: 0 });
                  setLlmTestStatus("fail");
                }
              }}
            >
              {llmTestStatus === "testing" ? (
                <Loader2 className="mr-1.5 h-3.5 w-3.5 animate-spin" />
              ) : null}
              测试连接
            </Button>
            {llmTestStatus === "ok" && llmTestResult && (
              <span className="flex items-center gap-1 text-xs text-green-600">
                <CheckCircle2 className="h-3.5 w-3.5" />
                {llmTestResult.message}
              </span>
            )}
            {llmTestStatus === "fail" && llmTestResult && (
              <span className="flex items-center gap-1 text-xs text-destructive">
                <XCircle className="h-3.5 w-3.5" />
                {llmTestResult.message}
              </span>
            )}
          </div>
        </CardContent>
      </Card>

      {/* ASR 配置 */}
      <Card>
        <CardHeader>
          <CardTitle className="text-xs font-semibold text-muted-foreground uppercase tracking-wide">
            ASR 配置
          </CardTitle>
        </CardHeader>
        <Separator />
        <CardContent className="space-y-4">
          {/* Provider 选择 */}
          <div className="space-y-1.5">
            <label className="text-sm font-medium text-foreground">ASR 引擎</label>
            <Select
              value={local.asr_provider}
              onValueChange={(v) => {
                setLocal({ ...local, asr_provider: v as AsrProviderType });
                setWhisperCheck(null);
                setAsrTestResult(null);
              }}
            >
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="local_whisper">本地 Whisper</SelectItem>
                <SelectItem value="aliyun">阿里云 ASR</SelectItem>
              </SelectContent>
            </Select>
          </div>

          {/* 识别语言（公共） */}
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

          {/* 本地 Whisper 面板 */}
          {local.asr_provider === "local_whisper" && (
            <>
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
                <label className="text-sm font-medium text-foreground">whisper-cli 路径</label>
                <div className="flex gap-2">
                  <Input
                    value={local.whisper_cli_path}
                    onChange={(e) => {
                      setLocal({ ...local, whisper_cli_path: e.target.value });
                      setWhisperCheck(null);
                    }}
                    placeholder="whisper-cli 或绝对路径"
                    className="flex-1"
                  />
                  <Button
                    variant="outline"
                    size="sm"
                    disabled={whisperChecking}
                    onClick={async () => {
                      setWhisperChecking(true);
                      setWhisperCheck(null);
                      try {
                        const result = await checkWhisperCli(local.whisper_cli_path);
                        setWhisperCheck(result);
                      } catch (e) {
                        setWhisperCheck({ found: false, version: null, message: String(e) });
                      } finally {
                        setWhisperChecking(false);
                      }
                    }}
                  >
                    {whisperChecking ? <Loader2 className="h-3.5 w-3.5 animate-spin" /> : "检测"}
                  </Button>
                </div>
                {whisperCheck && (
                  <p className={`flex items-center gap-1 text-xs ${whisperCheck.found ? "text-green-600" : "text-destructive"}`}>
                    {whisperCheck.found ? (
                      <CheckCircle2 className="h-3.5 w-3.5" />
                    ) : (
                      <XCircle className="h-3.5 w-3.5" />
                    )}
                    {whisperCheck.found && whisperCheck.version
                      ? whisperCheck.version
                      : whisperCheck.message}
                  </p>
                )}
                {!whisperCheck && (
                  <p className="text-[11px] text-muted-foreground">
                    下载：github.com/ggerganov/whisper.cpp/releases
                  </p>
                )}
              </div>

              <div className="space-y-1.5">
                <label className="text-sm font-medium text-foreground">模型文件目录</label>
                <Input
                  value={local.whisper_model_dir}
                  onChange={(e) => setLocal({ ...local, whisper_model_dir: e.target.value })}
                  placeholder="models"
                />
                <p className="text-[11px] text-muted-foreground">
                  存放 ggml-*.bin 模型文件的目录路径
                </p>
              </div>
            </>
          )}

          {/* 阿里云 ASR 面板 */}
          {local.asr_provider === "aliyun" && (
            <>
              <div className="space-y-1.5">
                <label className="text-sm font-medium text-foreground">AppKey</label>
                <Input
                  value={local.aliyun_asr_app_key}
                  onChange={(e) => setLocal({ ...local, aliyun_asr_app_key: e.target.value })}
                  placeholder="项目 AppKey"
                />
              </div>

              <div className="space-y-1.5">
                <label className="text-sm font-medium text-foreground">AccessKey ID</label>
                <Input
                  value={local.aliyun_asr_access_key_id}
                  onChange={(e) => setLocal({ ...local, aliyun_asr_access_key_id: e.target.value })}
                  placeholder="AccessKey ID"
                />
              </div>

              <div className="space-y-1.5">
                <label className="text-sm font-medium text-foreground">AccessKey Secret</label>
                <div className="flex gap-2">
                  <Input
                    type={showAliyunSecret ? "text" : "password"}
                    value={local.aliyun_asr_access_key_secret}
                    onChange={(e) => setLocal({ ...local, aliyun_asr_access_key_secret: e.target.value })}
                    placeholder="AccessKey Secret"
                    className="flex-1"
                  />
                  <Button
                    variant="ghost"
                    size="sm"
                    onClick={() => setShowAliyunSecret((v) => !v)}
                  >
                    {showAliyunSecret ? <EyeOff className="h-4 w-4" /> : <Eye className="h-4 w-4" />}
                  </Button>
                </div>
                <p className="text-[11px] text-muted-foreground">
                  在阿里云控制台 → 智能语音交互 → 项目管理 获取 AppKey；在账号中心获取 AccessKey
                </p>
              </div>

              <div className="flex items-center gap-3 pt-1">
                <Button
                  variant="outline"
                  size="sm"
                  disabled={asrTesting}
                  onClick={async () => {
                    setAsrTesting(true);
                    setAsrTestResult(null);
                    try {
                      const result = await testAsrConnection(local);
                      setAsrTestResult(result);
                    } catch (e) {
                      setAsrTestResult({ success: false, message: String(e) });
                    } finally {
                      setAsrTesting(false);
                    }
                  }}
                >
                  {asrTesting ? <Loader2 className="mr-1.5 h-3.5 w-3.5 animate-spin" /> : null}
                  测试配置
                </Button>
                {asrTestResult && (
                  <span className={`flex items-center gap-1 text-xs ${asrTestResult.success ? "text-green-600" : "text-destructive"}`}>
                    {asrTestResult.success ? (
                      <CheckCircle2 className="h-3.5 w-3.5" />
                    ) : (
                      <XCircle className="h-3.5 w-3.5" />
                    )}
                    {asrTestResult.message}
                  </span>
                )}
              </div>
            </>
          )}
        </CardContent>
      </Card>

      <Button onClick={handleSave} className="w-full" size="lg">
        {saved ? (
          <>
            <Check className="mr-2 h-4 w-4" />
            {t("settings.saved")}
          </>
        ) : (
          t("settings.save")
        )}
      </Button>
      {saveError && (
        <p className="text-sm text-destructive text-center">{saveError}</p>
      )}
    </div>
    </div>
  );
}
