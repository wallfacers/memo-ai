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
import { Check } from "lucide-react";
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
        </CardContent>
      </Card>

      {/* ASR 配置 */}
      <Card>
        <CardHeader>
          <CardTitle className="text-xs font-semibold text-muted-foreground uppercase tracking-wide">
            {t("settings.asr.sectionTitle")}
          </CardTitle>
        </CardHeader>
        <Separator />
        <CardContent className="space-y-4">
          <div className="space-y-1.5">
            <label className="text-sm font-medium text-foreground">{t("settings.asr.whisperModel")}</label>
            <Select
              value={local.whisper_model}
              onValueChange={(v) => setLocal({ ...local, whisper_model: v })}
            >
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="tiny">{t("settings.asr.modelTiny")}</SelectItem>
                <SelectItem value="base">{t("settings.asr.modelBase")}</SelectItem>
                <SelectItem value="small">{t("settings.asr.modelSmall")}</SelectItem>
                <SelectItem value="medium">{t("settings.asr.modelMedium")}</SelectItem>
                <SelectItem value="large">{t("settings.asr.modelLarge")}</SelectItem>
              </SelectContent>
            </Select>
          </div>

          <div className="space-y-1.5">
            <label className="text-sm font-medium text-foreground">{t("settings.asr.language")}</label>
            <Select
              value={local.language}
              onValueChange={(v) => setLocal({ ...local, language: v })}
            >
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="zh">{t("settings.asr.langZh")}</SelectItem>
                <SelectItem value="en">{t("settings.asr.langEn")}</SelectItem>
                <SelectItem value="auto">{t("settings.asr.langAuto")}</SelectItem>
              </SelectContent>
            </Select>
          </div>

          <div className="space-y-1.5">
            <label className="text-sm font-medium text-foreground">
              {t("settings.asr.whisperCliPath")}
            </label>
            <Input
              value={local.whisper_cli_path}
              onChange={(e) =>
                setLocal({ ...local, whisper_cli_path: e.target.value })
              }
              placeholder={t("settings.asr.whisperCliPathPlaceholder")}
            />
            <p className="text-[11px] text-muted-foreground">
              {t("settings.asr.whisperCliPathHint")}
            </p>
          </div>

          <div className="space-y-1.5">
            <label className="text-sm font-medium text-foreground">
              {t("settings.asr.modelDir")}
            </label>
            <Input
              value={local.whisper_model_dir}
              onChange={(e) =>
                setLocal({ ...local, whisper_model_dir: e.target.value })
              }
              placeholder={t("settings.asr.modelDirPlaceholder")}
            />
            <p className="text-[11px] text-muted-foreground">
              {t("settings.asr.modelDirHint")}
            </p>
          </div>
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
