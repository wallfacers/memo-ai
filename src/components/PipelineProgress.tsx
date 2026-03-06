import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { CheckCircle2, Loader2 } from "lucide-react";
import { useMeetingStore } from "@/store/meetingStore";
import type { PipelineStageDoneEvent } from "@/types";

const STAGE_NAMES: Record<number, string> = {
  1: "文本清洗",
  2: "说话人整理",
  3: "结构化提取",
  4: "会议总结",
  5: "行动项提取",
  6: "报告生成",
};

const TOTAL_STAGES = 6;

export function PipelineProgress() {
  const { pipelineStages, appendPipelineStage, recordingPhase } = useMeetingStore();

  useEffect(() => {
    const unlisten = listen<PipelineStageDoneEvent>("pipeline-stage-done", (event) => {
      appendPipelineStage(event.payload);
    });
    return () => {
      void unlisten.then((fn) => fn());
    };
  }, [appendPipelineStage]);

  const completedStageNums = new Set(pipelineStages.map((s) => s.stage));

  return (
    <div className="w-full max-w-lg rounded-lg border border-border bg-muted/30 px-4 py-3 space-y-2">
      <p className="text-xs font-medium text-foreground">AI 分析进度</p>
      <div className="space-y-1">
        {Array.from({ length: TOTAL_STAGES }, (_, i) => {
          const stageNum = i + 1;
          const done = completedStageNums.has(stageNum);
          const isActive =
            recordingPhase === "pipeline" &&
            !done &&
            stageNum === (pipelineStages.length + 1);
          const stageName = STAGE_NAMES[stageNum] ?? `阶段 ${stageNum}`;
          const stageData = pipelineStages.find((s) => s.stage === stageNum);

          return (
            <div key={stageNum} className="flex items-center gap-2 text-xs">
              {done ? (
                <CheckCircle2 className="h-3.5 w-3.5 shrink-0 text-green-600" />
              ) : isActive ? (
                <Loader2 className="h-3.5 w-3.5 shrink-0 animate-spin text-muted-foreground" />
              ) : (
                <span className="h-3.5 w-3.5 shrink-0 rounded-full border border-muted-foreground/30" />
              )}
              <span className={done ? "text-foreground" : "text-muted-foreground"}>
                {stageName}
              </span>
              {done && stageData && (
                <span className="ml-auto text-muted-foreground">
                  {stageData.elapsed_ms}ms
                </span>
              )}
            </div>
          );
        })}
      </div>
    </div>
  );
}
