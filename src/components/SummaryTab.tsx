import { useState, useRef, useEffect, useCallback } from "react";
import { useTranslation } from "react-i18next";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { Pencil, Check, Copy, RefreshCw, Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useUpdateMeetingSummary, useRegenerateSummaryStream } from "@/hooks/useTauriCommands";
import type { Meeting } from "@/types";

interface SummaryTabProps {
  meeting: Meeting;
  onSummaryUpdated: (newSummary: string) => void;
}

type RegeneratePhase = "idle" | "stage1" | "stage2" | "stage4" | "streaming";

export function SummaryTab({ meeting, onSummaryUpdated }: SummaryTabProps) {
  const { t } = useTranslation();
  const [isEditing, setIsEditing] = useState(false);
  const [editText, setEditText] = useState(meeting.summary ?? "");
  const [isCopied, setIsCopied] = useState(false);
  const [regenPhase, setRegenPhase] = useState<RegeneratePhase>("idle");
  const [streamingText, setStreamingText] = useState("");
  const debounceTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const unlistenRef = useRef<UnlistenFn[]>([]);
  const updateMeetingSummary = useUpdateMeetingSummary();
  const regenerateSummaryStream = useRegenerateSummaryStream();

  // 当外部 meeting.summary 更新时（如重新生成后），同步 editText
  useEffect(() => {
    if (!isEditing) {
      setEditText(meeting.summary ?? "");
    }
  }, [meeting.summary, isEditing]);

  const saveWithDebounce = useCallback((text: string) => {
    if (debounceTimerRef.current) {
      clearTimeout(debounceTimerRef.current);
    }
    debounceTimerRef.current = setTimeout(() => {
      void updateMeetingSummary(meeting.id, text).then(() => {
        onSummaryUpdated(text);
      });
    }, 1000);
  }, [meeting.id, updateMeetingSummary, onSummaryUpdated]);

  // 卸载时取消 debounce timer 并清理事件监听
  useEffect(() => {
    return () => {
      if (debounceTimerRef.current) {
        clearTimeout(debounceTimerRef.current);
      }
      unlistenRef.current.forEach((fn) => fn());
      unlistenRef.current = [];
    };
  }, []);

  function handleEditToggle() {
    if (isEditing) {
      if (debounceTimerRef.current) {
        clearTimeout(debounceTimerRef.current);
        debounceTimerRef.current = null;
      }
      void updateMeetingSummary(meeting.id, editText).then(() => {
        onSummaryUpdated(editText);
      });
    }
    setIsEditing((prev) => !prev);
  }

  function handleTextChange(e: React.ChangeEvent<HTMLTextAreaElement>) {
    const text = e.target.value;
    setEditText(text);
    saveWithDebounce(text);
  }

  async function handleCopy() {
    const text = isEditing ? editText : (meeting.summary ?? "");
    try {
      await navigator.clipboard.writeText(text);
      setIsCopied(true);
      setTimeout(() => setIsCopied(false), 1500);
    } catch (e) {
      console.error("Failed to copy to clipboard:", e);
    }
  }

  async function handleRegenerate() {
    // 清理上次监听
    unlistenRef.current.forEach((fn) => fn());
    unlistenRef.current = [];
    setStreamingText("");
    setRegenPhase("stage1");

    const unlistenStage = await listen<{ stage: number; name: string }>(
      "summary_stage",
      (event) => {
        const s = event.payload.stage;
        if (s === 1) setRegenPhase("stage1");
        else if (s === 2) setRegenPhase("stage2");
        else if (s === 4) setRegenPhase("stage4");
      }
    );
    unlistenRef.current = [unlistenStage];

    const unlistenChunk = await listen<{ text: string }>(
      "summary_chunk",
      (event) => {
        setRegenPhase("streaming");
        setStreamingText((prev) => prev + event.payload.text);
      }
    );
    unlistenRef.current = [unlistenStage, unlistenChunk];

    const unlistenDone = await listen<{ summary: string }>(
      "summary_done",
      (event) => {
        setStreamingText(event.payload.summary);
        setEditText(event.payload.summary);
        onSummaryUpdated(event.payload.summary);
        unlistenRef.current.forEach((fn) => fn());
        unlistenRef.current = [];
        setRegenPhase("idle");
      }
    );
    unlistenRef.current = [unlistenStage, unlistenChunk, unlistenDone];

    const unlistenError = await listen<{ message: string }>(
      "summary_error",
      (event) => {
        console.error("Regenerate summary failed:", event.payload.message);
        setRegenPhase("idle");
        setStreamingText("");
        unlistenRef.current.forEach((fn) => fn());
        unlistenRef.current = [];
      }
    );
    unlistenRef.current = [unlistenStage, unlistenChunk, unlistenDone, unlistenError];

    try {
      await regenerateSummaryStream(meeting.id);
    } catch (e) {
      console.error("Failed to invoke regenerate_summary_stream:", e);
      setRegenPhase("idle");
      unlistenRef.current.forEach((fn) => fn());
      unlistenRef.current = [];
    }
  }

  const isProcessing = meeting.status === "processing";
  const isRegenerating = regenPhase !== "idle";
  const hasSummary = !!meeting.summary;

  const stageLabel =
    regenPhase === "stage1" ? t("summary.actions.stage1") :
    regenPhase === "stage2" ? t("summary.actions.stage2") :
    regenPhase === "stage4" ? t("summary.actions.stage4") : "";

  return (
    <div className="flex flex-col gap-2">
      {/* 工具栏 */}
      <div className="flex justify-start gap-1 px-4">
        {/* 编辑 / 完成 */}
        <Button
          variant="ghost"
          size="icon"
          className="h-7 w-7 rounded-md text-muted-foreground hover:text-foreground"
          title={isEditing ? t("summary.actions.finishEdit") : t("summary.actions.edit")}
          disabled={isProcessing || isRegenerating || !hasSummary}
          onClick={handleEditToggle}
        >
          {isEditing
            ? <Check className="h-4 w-4 text-green-500" />
            : <Pencil className="h-4 w-4" />
          }
        </Button>

        {/* 复制 */}
        <Button
          variant="ghost"
          size="icon"
          className="h-7 w-7 rounded-md text-muted-foreground hover:text-foreground"
          title={t("summary.actions.copy")}
          disabled={isProcessing || isRegenerating || !hasSummary}
          onClick={handleCopy}
        >
          {isCopied
            ? <Check className="h-4 w-4 text-green-500" />
            : <Copy className="h-4 w-4" />
          }
        </Button>

        {/* 重新生成 */}
        <Button
          variant="ghost"
          size="icon"
          className="h-7 w-7 rounded-md text-muted-foreground hover:text-foreground"
          title={t("summary.actions.regenerate")}
          disabled={isProcessing || isRegenerating || isEditing}
          onClick={handleRegenerate}
        >
          {isRegenerating
            ? <Loader2 className="h-4 w-4 animate-spin" />
            : <RefreshCw className="h-4 w-4" />
          }
        </Button>
      </div>

      {/* 内容区域 */}
      {(regenPhase === "stage1" || regenPhase === "stage2" || regenPhase === "stage4") ? (
        <div className="flex items-center gap-2 px-4 py-12 text-sm text-muted-foreground">
          <Loader2 className="h-4 w-4 animate-spin shrink-0" />
          <span>{stageLabel}</span>
        </div>
      ) : regenPhase === "streaming" ? (
        <div className="px-4 py-2 text-sm font-mono leading-relaxed text-foreground whitespace-pre-wrap">
          {streamingText}
          <span className="inline-block w-0.5 h-4 bg-foreground ml-0.5 animate-pulse" />
        </div>
      ) : isEditing ? (
        <textarea
          className="w-full min-h-[200px] resize-y rounded-md border border-border bg-background px-3 py-2 text-sm font-mono leading-relaxed text-foreground focus:outline-none focus:ring-1 focus:ring-ring"
          value={editText}
          onChange={handleTextChange}
          autoFocus
        />
      ) : meeting.summary ? (
        <div className="p-4 text-sm leading-relaxed text-foreground prose prose-sm max-w-none dark:prose-invert">
          <ReactMarkdown remarkPlugins={[remarkGfm]}>
            {meeting.summary}
          </ReactMarkdown>
        </div>
      ) : (
        <div className="flex items-center justify-center py-12 text-sm text-muted-foreground">
          {t("meeting.noSummary")}
        </div>
      )}
    </div>
  );
}
