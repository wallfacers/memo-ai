import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { Progress } from "@/components/ui/progress";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent } from "@/components/ui/card";
import { Separator } from "@/components/ui/separator";
import { Loader2, CheckCircle2 } from "lucide-react";
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

export function PipelineProgress() {
  const {
    pipelineStages,
    appendPipelineStage,
    recordingPhase,
  } = useMeetingStore();

  // 监听 Pipeline 阶段完成事件
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    listen<PipelineStageDoneEvent>("pipeline_stage_done", (event) => {
      appendPipelineStage(event.payload);
    }).then((fn) => {
      unlisten = fn;
    });
    return () => {
      unlisten?.();
    };
  }, [appendPipelineStage]);

  // 仅在 pipeline 或 done 阶段显示
  if (recordingPhase !== "pipeline" && recordingPhase !== "done" && pipelineStages.length === 0) {
    return null;
  }

  const completedCount = pipelineStages.length;
  const progress = Math.round((completedCount / 6) * 100);
  // 当前正在执行的 stage（completedCount + 1，若 < 7）
  const currentStageNum = completedCount < 6 ? completedCount + 1 : null;

  return (
    <Card className="mt-3">
      <CardContent className="pt-4 space-y-3">
        {/* 总进度条 */}
        <div className="space-y-1">
          <div className="flex justify-between text-xs text-muted-foreground">
            <span>
              {completedCount < 6
                ? currentStageNum
                  ? `AI 分析中（${STAGE_NAMES[currentStageNum]}）`
                  : "AI 分析中…"
                : "分析完成"}
            </span>
            <span>{completedCount} / 6</span>
          </div>
          <Progress value={progress} className="h-1.5" />
        </div>

        <Separator />

        {/* 各阶段状态列表 */}
        <div className="space-y-2">
          {Object.entries(STAGE_NAMES).map(([numStr, name]) => {
            const stageNum = Number(numStr);
            const doneStage = pipelineStages.find((s) => s.stage === stageNum);
            const isRunning =
              !doneStage &&
              stageNum === currentStageNum &&
              recordingPhase === "pipeline";
            const isPending = !doneStage && !isRunning;

            return (
              <div key={stageNum} className="flex items-start gap-2">
                {/* 状态 Badge */}
                {doneStage ? (
                  <Badge
                    variant="default"
                    className="shrink-0 text-xs py-0 px-2 gap-1"
                  >
                    <CheckCircle2 className="h-3 w-3" />
                    {name}
                  </Badge>
                ) : isRunning ? (
                  <Badge
                    variant="secondary"
                    className="shrink-0 text-xs py-0 px-2 gap-1"
                  >
                    <Loader2 className="h-3 w-3 animate-spin" />
                    {name}
                  </Badge>
                ) : (
                  <Badge
                    variant="outline"
                    className="shrink-0 text-xs py-0 px-2 text-muted-foreground"
                  >
                    {name}
                  </Badge>
                )}

                {/* 结果摘要 */}
                {doneStage && (
                  <span className="text-xs text-muted-foreground leading-5 line-clamp-2">
                    {doneStage.summary}
                  </span>
                )}
                {isRunning && (
                  <span className="flex items-center gap-1 text-xs text-muted-foreground leading-5">
                    正在生成
                    <span className="inline-block w-px h-3.5 bg-muted-foreground animate-pulse" />
                  </span>
                )}
                {isPending && (
                  <span className="text-xs text-muted-foreground/50 leading-5">
                    等待中
                  </span>
                )}
              </div>
            );
          })}
        </div>
      </CardContent>
    </Card>
  );
}
