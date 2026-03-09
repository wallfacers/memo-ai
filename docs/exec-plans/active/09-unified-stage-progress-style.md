# 统一进度条视觉风格 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 将 `SummaryTab` 重新生成进度从单行 spinner 升级为 3 阶段清单，与 `PipelineProgress` 的视觉风格保持一致。

**Background:**
- `PipelineProgress`（首次处理）：6 行清单，全部阶段同时可见，4 态（done/failed/active/pending）
- `SummaryTab` 重新生成：仅显示单行当前阶段名 + spinner，其他阶段不可见，信息密度低
- 目标：`SummaryTab` 进度区也改为全阶段清单，视觉语言一致

**Architecture:**
- 提取纯展示层 `StageProgressList` 组件（仅 done/active/pending 三态，无 failed），专供 `SummaryTab` 使用
- `PipelineProgress` **不动**：它已实现 failed 态 + 重试按钮 + 错误文本，远超 `StageProgressList` 能力，重构会 regression plan-08 功能
- 两处视觉一致通过"SummaryTab 外观向 PipelineProgress 看齐"实现，而非强制共用组件

**Tech Stack:** React / TypeScript / Tailwind CSS / lucide-react

---

## Task 1: 创建 StageProgressList 组件

**Files:**
- Create: `src/components/StageProgressList.tsx`

**Step 1: 创建组件**

```typescript
import { CheckCircle2, Loader2 } from "lucide-react";

export interface StageItem {
  key: string | number;
  label: string;
  status: "done" | "active" | "pending";
}

interface StageProgressListProps {
  stages: StageItem[];
  title?: string;
}

export function StageProgressList({ stages, title }: StageProgressListProps) {
  return (
    <div className="w-full rounded-lg border border-border bg-muted/30 px-4 py-3 space-y-2">
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
          </div>
        ))}
      </div>
    </div>
  );
}
```

注意：
- 不包含 `"failed"` 状态——`SummaryTab` 重新生成没有失败态（失败直接回到 idle），此处无需
- 不包含 `elapsedMs`——重新生成不记录耗时
- 不包含 retry 按钮——这是 `PipelineProgress` 专属逻辑

**Step 2: 类型检查**

```bash
npx tsc --noEmit 2>&1 | tail -5
```

Expected: 无 error。

**Step 3: Commit**

```bash
git add src/components/StageProgressList.tsx
git commit -m "feat(ui): add StageProgressList shared component for stage progress display"
```

---

## Task 2: 更新 SummaryTab 使用 StageProgressList

**Files:**
- Modify: `src/components/SummaryTab.tsx`

**Step 1: 添加 import**

在文件顶部 import 区添加：

```typescript
import { StageProgressList, type StageItem } from "@/components/StageProgressList";
```

同时移除不再需要的 `Loader2` import（确认其他地方没有用到后再删）。

**Step 2: 删除 stageLabel 变量**

删除第 155-158 行的 `stageLabel` 计算：

```typescript
// 删除以下代码：
const stageLabel =
  regenPhase === "stage1" ? t("summary.actions.stage1") :
  regenPhase === "stage2" ? t("summary.actions.stage2") :
  regenPhase === "stage4" ? t("summary.actions.stage4") : "";
```

**Step 3: 在 `isProcessing` 附近添加 regenStages 计算**

在 `const isProcessing = ...` 之后添加：

```typescript
const REGEN_STAGES: Array<{ phase: RegeneratePhase; label: string }> = [
  { phase: "stage1", label: "文本清洗" },
  { phase: "stage2", label: "说话人整理" },
  { phase: "stage4", label: "生成总结" },
];

const activeRegenIdx = REGEN_STAGES.findIndex((s) => s.phase === regenPhase);

const regenStages: StageItem[] = REGEN_STAGES.map((s, i) => ({
  key: s.phase,
  label: s.label,
  status:
    activeRegenIdx < 0 || i < activeRegenIdx
      ? "done"
      : i === activeRegenIdx
      ? "active"
      : "pending",
}));
```

注意：`activeRegenIdx < 0` 表示 regenPhase 为 "idle" 或 "streaming"，此时 regenStages 不会被渲染（条件判断在 Step 4 中处理）。

**Step 4: 替换内容区域的阶段进度渲染**

将：

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

**Step 5: 类型检查**

```bash
npx tsc --noEmit 2>&1 | tail -10
```

Expected: 无 error。

**Step 6: Commit**

```bash
git add src/components/SummaryTab.tsx
git commit -m "refactor(SummaryTab): replace single-line spinner with StageProgressList for regen progress"
```

---

## Task 3: 清理废弃的 i18n key

**Files:**
- Modify: `src/i18n/locales/zh.ts`
- Modify: `src/i18n/locales/en.ts`

**Step 1: 确认 stage1/2/4 key 不再被使用**

```bash
grep -r "summary.actions.stage" src/
```

Expected: 只出现在 i18n 文件中，不出现在 .tsx/.ts 组件文件里（Task 2 已删除 stageLabel）。

**Step 2: 删除 zh.ts 中的废弃 key**

在 `src/i18n/locales/zh.ts` 的 `summary.actions` 对象中，删除：

```typescript
stage1: "正在清洗文本...",
stage2: "正在整理说话人...",
stage4: "正在生成摘要...",
```

**Step 3: 删除 en.ts 中的废弃 key**

在 `src/i18n/locales/en.ts` 的 `summary.actions` 对象中，删除对应的 stage1/2/4 key（名称相同，值为英文）。

**Step 4: 类型检查**

```bash
npx tsc --noEmit 2>&1 | tail -5
```

Expected: 无 error。

**Step 5: Commit**

```bash
git add src/i18n/locales/zh.ts src/i18n/locales/en.ts
git commit -m "chore(i18n): remove unused summary.actions.stage1/2/4 keys"
```

---

## Task 4: 手动验证

**Step 1: 启动开发模式**

```bash
npm run tauri:dev
```

**Step 2: SummaryTab 重新生成验证**

- 打开已有转写内容的会议，切换到「总结」Tab
- 点击重新生成按钮
- 观察内容区：应显示 3 行清单（文本清洗 / 说话人整理 / 生成总结）
- 当前活跃阶段显示 spinner，已完成阶段显示绿色 checkmark，待处理阶段显示空心圆
- 进入 streaming 阶段后，清单消失，流式文本接管内容区

**Step 3: PipelineProgress 回归验证**

- 录制新会议并停止
- 确认 Pipeline 进度条功能完全不受影响（6 阶段、重试按钮、错误文本均正常）

**验收标准：**

- [ ] SummaryTab 重新生成进度从单行文字改为 3 阶段清单
- [ ] 与 PipelineProgress 视觉风格一致（字号、图标大小、颜色、容器样式）
- [ ] streaming 阶段正常切换到流式文本输出
- [ ] PipelineProgress 功能与外观完全不受影响
- [ ] TypeScript 无报错
- [ ] i18n 无废弃 key

---

## 文件变更汇总

| 文件 | Task | 变更类型 |
|------|------|---------|
| `src/components/StageProgressList.tsx` | Task 1 | 新建 |
| `src/components/SummaryTab.tsx` | Task 2 | 修改（进度展示升级） |
| `src/i18n/locales/zh.ts` | Task 3 | 删除废弃 key |
| `src/i18n/locales/en.ts` | Task 3 | 删除废弃 key |

**不修改：**
- `src/components/PipelineProgress.tsx`（已包含 failed 态 + 重试逻辑，远超 StageProgressList 能力）
