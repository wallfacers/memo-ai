# 统一进度条视觉风格 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 提取公共 `StageProgressList` 展示组件，统一 `PipelineProgress`（首次处理）和 `SummaryTab`（重新生成）两处 AI 处理进度的视觉语言，使用户在两种场景下看到一致的阶段清单样式。

**Background:** 经过评估，两个进度功能业务逻辑不能合并（触发时机、事件来源、位置不同），但视觉风格可以也应该统一。当前差异：
- `PipelineProgress`：清单列表，所有阶段同时可见，有 checkmark/spinner/pending 三态
- `SummaryTab` 重新生成：仅显示当前阶段的单行文字，其他阶段不可见，信息密度低

**Architecture:** 提取纯展示层 `StageProgressList` 组件，`PipelineProgress` 和 `SummaryTab` 各自保持独立业务逻辑，共用同一视觉组件。

**Tech Stack:** React / TypeScript / Tailwind CSS / lucide-react

---

## Task 1: 提取 StageProgressList 组件

**Files:**
- Create: `src/components/StageProgressList.tsx`

**Step 1: 创建组件**

新建文件，内容如下：

```typescript
import { CheckCircle2, Loader2 } from "lucide-react";

export interface StageItem {
  key: string | number;
  label: string;
  status: "done" | "active" | "pending";
  elapsedMs?: number;
}

interface StageProgressListProps {
  stages: StageItem[];
  title?: string;
}

export function StageProgressList({ stages, title }: StageProgressListProps) {
  return (
    <div className="w-full max-w-lg rounded-lg border border-border bg-muted/30 px-4 py-3 space-y-2">
      {title && <p className="text-xs font-medium text-foreground">{title}</p>}
      <div className="space-y-1">
        {stages.map((stage) => (
          <div key={stage.key} className="flex items-center gap-2 text-xs">
            {stage.status === "done" ? (
              <CheckCircle2 className="h-3.5 w-3.5 shrink-0 text-green-600" />
            ) : stage.status === "active" ? (
              <Loader2 className="h-3.5 w-3.5 shrink-0 animate-spin text-muted-foreground" />
            ) : (
              <span className="h-3.5 w-3.5 shrink-0 rounded-full border border-muted-foreground/30" />
            )}
            <span className={stage.status === "done" ? "text-foreground" : "text-muted-foreground"}>
              {stage.label}
            </span>
            {stage.status === "done" && stage.elapsedMs !== undefined && (
              <span className="ml-auto text-muted-foreground">{stage.elapsedMs}ms</span>
            )}
          </div>
        ))}
      </div>
    </div>
  );
}
```

**Step 2: 类型检查**

```bash
npx tsc --noEmit 2>&1 | tail -5
```

Expected: 无 error。

**Step 3: Commit**

```bash
git add src/components/StageProgressList.tsx
git commit -m "feat(ui): add StageProgressList shared component"
```

---

## Task 2: 改造 PipelineProgress 使用 StageProgressList

**Files:**
- Modify: `src/components/PipelineProgress.tsx`

**Step 1: 理解当前实现**

当前组件在内部直接渲染阶段列表（CheckCircle2/Loader2/空心圆），需要改为调用 `StageProgressList`，传入构造好的 `StageItem[]`。

**Step 2: 替换实现**

将文件内容替换为：

```typescript
import { useEffect } from "react";
import { listen } from "@tauri-apps/api/event";
import { useMeetingStore } from "@/store/meetingStore";
import type { PipelineStageDoneEvent } from "@/types";
import { StageProgressList, type StageItem } from "@/components/StageProgressList";

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

  const stages: StageItem[] = Array.from({ length: TOTAL_STAGES }, (_, i) => {
    const stageNum = i + 1;
    const done = completedStageNums.has(stageNum);
    const isActive =
      recordingPhase === "pipeline" &&
      !done &&
      stageNum === pipelineStages.length + 1;
    const stageData = pipelineStages.find((s) => s.stage === stageNum);

    return {
      key: stageNum,
      label: STAGE_NAMES[stageNum] ?? `阶段 ${stageNum}`,
      status: done ? "done" : isActive ? "active" : "pending",
      elapsedMs: stageData?.elapsed_ms,
    };
  });

  return <StageProgressList stages={stages} title="AI 分析进度" />;
}
```

**Step 3: 编译验证**

```bash
npx tsc --noEmit 2>&1 | tail -5
```

Expected: 无 error。

**Step 4: Commit**

```bash
git add src/components/PipelineProgress.tsx
git commit -m "refactor(PipelineProgress): use StageProgressList for unified style"
```

---

## Task 3: 改造 SummaryTab 重新生成进度使用 StageProgressList

**Files:**
- Modify: `src/components/SummaryTab.tsx`

**Step 1: 理解当前实现**

当前在 `regenPhase === "stage1" | "stage2" | "stage4"` 时，渲染：
```tsx
<div className="flex items-center gap-2 px-4 py-12 ...">
  <Loader2 ... />
  <span>{stageLabel}</span>
</div>
```

只显示单行当前阶段，其他阶段不可见。改为显示 3 阶段清单：
- stage 1：文本清洗
- stage 2：说话人整理
- stage 4（逻辑上第 3 步）：生成总结

**Step 2: 新增 import**

在文件顶部 import 区添加：
```typescript
import { StageProgressList, type StageItem } from "@/components/StageProgressList";
```

**Step 3: 新增 regenStages 计算逻辑**

在 `const isProcessing = ...` 附近，添加以下计算：

```typescript
const REGEN_STAGE_ORDER: Array<{ phase: RegeneratePhase; label: string }> = [
  { phase: "stage1", label: "文本清洗" },
  { phase: "stage2", label: "说话人整理" },
  { phase: "stage4", label: "生成总结" },
];

const regenStageIndex = REGEN_STAGE_ORDER.findIndex((s) => s.phase === regenPhase);

const regenStages: StageItem[] = REGEN_STAGE_ORDER.map((s, i) => ({
  key: s.phase,
  label: s.label,
  status:
    regenStageIndex < 0 || i < regenStageIndex
      ? "done"
      : i === regenStageIndex
      ? "active"
      : "pending",
}));
```

**Step 4: 替换渲染逻辑**

找到内容区域的条件渲染，将：
```tsx
{(regenPhase === "stage1" || regenPhase === "stage2" || regenPhase === "stage4") ? (
  <div className="flex items-center gap-2 px-4 py-12 text-sm text-muted-foreground">
    <Loader2 className="h-4 w-4 animate-spin shrink-0" />
    <span>{stageLabel}</span>
  </div>
) : ...}
```

替换为：
```tsx
{(regenPhase === "stage1" || regenPhase === "stage2" || regenPhase === "stage4") ? (
  <div className="px-4 py-4">
    <StageProgressList stages={regenStages} title="重新生成中" />
  </div>
) : ...}
```

**Step 5: 清理不再使用的变量**

删除 `stageLabel` 变量（已不使用）：
```typescript
// 删除这段：
const stageLabel =
  regenPhase === "stage1" ? t("summary.actions.stage1") :
  regenPhase === "stage2" ? t("summary.actions.stage2") :
  regenPhase === "stage4" ? t("summary.actions.stage4") : "";
```

检查 `t("summary.actions.stage1/2/4")` 是否仅在 `stageLabel` 中使用，确认可以安全删除。

**Step 6: 编译验证**

```bash
npx tsc --noEmit 2>&1 | tail -5
```

Expected: 无 error。

**Step 7: Commit**

```bash
git add src/components/SummaryTab.tsx
git commit -m "refactor(SummaryTab): use StageProgressList for regen progress, show all stages"
```

---

## Task 4: 手动验证

**测试步骤：**

1. 运行 `npm run tauri:dev`

2. **PipelineProgress 验证（首次处理）：**
   - 录制一段音频并停止
   - 观察处理进度：确认 6 个阶段以清单形式展示，样式与之前一致
   - 完成后各阶段显示绿色 checkmark 和耗时

3. **SummaryTab 重新生成验证：**
   - 打开一个已有总结的会议，切换到"总结"Tab
   - 点击重新生成按钮
   - 观察内容区：应显示 3 阶段清单（文本清洗 / 说话人整理 / 生成总结），与 PipelineProgress 视觉风格一致
   - 确认当前活跃阶段显示 spinner，已完成阶段显示 checkmark，待处理阶段显示空心圆
   - 进入 streaming 阶段后，清单消失，流式文本接管内容区

**验收标准：**

- [ ] PipelineProgress 功能与外观不受影响
- [ ] SummaryTab 重新生成进度从单行文字改为 3 阶段清单
- [ ] 两处进度条视觉风格一致（字号、图标、颜色、容器样式）
- [ ] streaming 阶段正常切换到流式文本输出
- [ ] TypeScript 无报错

---

## 文件变更汇总

| 文件 | Task | 变更类型 |
|------|------|---------|
| `src/components/StageProgressList.tsx` | Task 1 | 新建 |
| `src/components/PipelineProgress.tsx` | Task 2 | 重构（使用共享组件） |
| `src/components/SummaryTab.tsx` | Task 3 | 重构（进度展示升级） |
