import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { CheckCircle2, Loader2, XCircle, RotateCcw } from "lucide-react";
import { useMeetingStore } from "@/store/meetingStore";
import type { PipelineStageDoneEvent, PipelineStageFailed } from "@/types";
import { useTranslation } from "react-i18next";

const STAGE_NAMES: Record<number, string> = {
  1: "文本清洗",
  2: "说话人整理",
  3: "结构化提取",
  4: "会议总结",
  5: "行动项提取",
  6: "报告生成",
};

const TOTAL_STAGES = 6;

interface PipelineProgressProps {
  onRetryFromStage: (stage: number) => void;
}

export function PipelineProgress({ onRetryFromStage }: PipelineProgressProps) {
  const {
    pipelineStages,
    appendPipelineStage,
    recordingPhase,
    pipelineFailedStage,
    setPipelineFailedStage,
  } = useMeetingStore();
  const { t } = useTranslation();

  useEffect(() => {
    const unlistenDone = listen<PipelineStageDoneEvent>("pipeline-stage-done", (event) => {
      appendPipelineStage(event.payload);
    });
    const unlistenFailed = listen<PipelineStageFailed>("pipeline-stage-failed", (event) => {
      setPipelineFailedStage(event.payload);
    });
    return () => {
      void unlistenDone.then((fn) => fn());
      void unlistenFailed.then((fn) => fn());
    };
  }, [appendPipelineStage, setPipelineFailedStage]);

  const completedStageNums = new Set(pipelineStages.map((s) => s.stage));
  const failedStageNum = pipelineFailedStage?.stage ?? null;

  return (
    <div className="w-full max-w-lg rounded-lg border border-border bg-muted/30 px-4 py-3 space-y-2">
      <p className="text-xs font-medium text-foreground">AI 分析进度</p>
      <div className="space-y-1">
        {Array.from({ length: TOTAL_STAGES }, (_, i) => {
          const stageNum = i + 1;
          const done = completedStageNums.has(stageNum);
          const isFailed = stageNum === failedStageNum;
          const isActive =
            recordingPhase === "pipeline" &&
            !done &&
            !isFailed &&
            stageNum === pipelineStages.length + 1;
          const stageName = STAGE_NAMES[stageNum] ?? `阶段 ${stageNum}`;
          const stageData = pipelineStages.find((s) => s.stage === stageNum);

          return (
            <div key={stageNum} className="space-y-0.5">
              <div className="flex items-center gap-2 text-xs">
                {done ? (
                  <CheckCircle2 className="h-3.5 w-3.5 shrink-0 text-green-600" />
                ) : isFailed ? (
                  <XCircle className="h-3.5 w-3.5 shrink-0 text-destructive" />
                ) : isActive ? (
                  <Loader2 className="h-3.5 w-3.5 shrink-0 animate-spin text-muted-foreground" />
                ) : (
                  <span className="h-3.5 w-3.5 shrink-0 rounded-full border border-muted-foreground/30" />
                )}
                <span
                  className={
                    isFailed
                      ? "text-destructive"
                      : done
                      ? "text-foreground"
                      : "text-muted-foreground"
                  }
                >
                  {stageName}
                </span>
                {done && stageData && (
                  <span className="ml-auto text-muted-foreground">
                    {stageData.elapsed_ms}ms
                  </span>
                )}
                {isFailed && (
                  <button
                    onClick={() => onRetryFromStage(stageNum)}
                    className="ml-auto flex items-center gap-1 text-xs text-muted-foreground hover:text-foreground transition-colors"
                  >
                    <RotateCcw className="h-3 w-3" />
                    {t("meeting.phase.retryFromStage")}
                  </button>
                )}
              </div>
              {isFailed && pipelineFailedStage?.error && (
                <p className="ml-5 text-[11px] text-destructive/70 leading-relaxed">
                  {pipelineFailedStage.error}
                </p>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
