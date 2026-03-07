import { useState, useRef, useEffect, useCallback } from "react";
import { useTranslation } from "react-i18next";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { Pencil, Check, Copy, RefreshCw, Loader2 } from "lucide-react";
import { Button } from "@/components/ui/button";
import { useUpdateMeetingSummary, useRegenerateSummary } from "@/hooks/useTauriCommands";
import type { Meeting } from "@/types";

interface SummaryTabProps {
  meeting: Meeting;
  onSummaryUpdated: (newSummary: string) => void;
}

export function SummaryTab({ meeting, onSummaryUpdated }: SummaryTabProps) {
  const { t } = useTranslation();
  const [isEditing, setIsEditing] = useState(false);
  const [editText, setEditText] = useState(meeting.summary ?? "");
  const [isCopied, setIsCopied] = useState(false);
  const [isRegenerating, setIsRegenerating] = useState(false);
  const debounceTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const updateMeetingSummary = useUpdateMeetingSummary();
  const regenerateSummary = useRegenerateSummary();

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

  // 卸载时取消未完成的 debounce timer
  useEffect(() => {
    return () => {
      if (debounceTimerRef.current) {
        clearTimeout(debounceTimerRef.current);
      }
    };
  }, []);

  function handleEditToggle() {
    if (isEditing) {
      // 退出编辑：立即保存当前内容
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
    await navigator.clipboard.writeText(text);
    setIsCopied(true);
    setTimeout(() => setIsCopied(false), 1500);
  }

  async function handleRegenerate() {
    setIsRegenerating(true);
    try {
      const newSummary = await regenerateSummary(meeting.id);
      setEditText(newSummary);
      onSummaryUpdated(newSummary);
    } catch (e) {
      console.error("Regenerate summary failed:", e);
    } finally {
      setIsRegenerating(false);
    }
  }

  const isProcessing = meeting.status === "processing";
  const hasSummary = !!meeting.summary;

  return (
    <div className="flex flex-col gap-2">
      {/* 工具栏 */}
      <div className="flex justify-end gap-1">
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
      {isEditing ? (
        <textarea
          className="w-full min-h-[200px] resize-y rounded-md border border-border bg-background px-3 py-2 text-sm font-mono leading-relaxed text-foreground focus:outline-none focus:ring-1 focus:ring-ring"
          value={editText}
          onChange={handleTextChange}
          autoFocus
        />
      ) : hasSummary ? (
        <div className="p-4 text-sm leading-relaxed text-foreground prose prose-sm max-w-none dark:prose-invert">
          <ReactMarkdown remarkPlugins={[remarkGfm]}>
            {meeting.summary!}
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
