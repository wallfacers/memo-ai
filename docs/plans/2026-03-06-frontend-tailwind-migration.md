# Frontend Tailwind + shadcn/ui 全量迁移 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 将所有内联样式迁移至 Tailwind CSS v4 + shadcn/ui，并重构为侧边栏双栏布局，支持系统跟随暗色模式。

**Architecture:** App Shell 采用左侧固定 240px 侧边栏 + 右侧 flex-1 内容区；Meeting 页改为 Tabs 布局；shadcn/ui 组件通过 CLI 安装，不手动修改 `src/components/ui/`。

**Tech Stack:** Tailwind CSS v4（`@tailwindcss/vite`），shadcn/ui（latest），lucide-react，class-variance-authority，clsx，tailwind-merge

---

## Task 1: 安装 Tailwind CSS v4 + 配置 Vite

**Files:**
- Modify: `vite.config.ts`
- Modify: `package.json`（通过 npm install 自动）

**Step 1: 安装 Tailwind CSS v4 及 Vite 插件**

```bash
cd D:/project/java/source/memo-ai
npm install tailwindcss @tailwindcss/vite
```

Expected: `added X packages` 无报错

**Step 2: 修改 `vite.config.ts`，注册 Tailwind 插件**

将文件改为：
```typescript
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

export default defineConfig(async () => ({
  plugins: [react(), tailwindcss()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
}));
```

**Step 3: 创建 `src/index.css`，引入 Tailwind**

```css
@import "tailwindcss";
```

**Step 4: 确认 `src/main.tsx` 引入了 CSS**

检查 `src/main.tsx` 是否有 `import "./index.css"`，如无则添加。

**Step 5: 验证 Tailwind 已生效**

```bash
npm run dev
```

Expected: 控制台无报错，浏览器打开 `http://localhost:1420`，页面字体应变为 sans-serif 系统字体。

**Step 6: Commit**

```bash
git add vite.config.ts src/index.css src/main.tsx package.json package-lock.json
git commit -m "chore: install tailwindcss v4 with vite plugin"
```

---

## Task 2: 安装 shadcn/ui + 初始化

**Files:**
- Create: `components.json`（shadcn 配置，自动生成）
- Modify: `src/index.css`（shadcn init 会注入 CSS 变量）
- Create: `src/lib/utils.ts`（shadcn 工具函数）

**Step 1: 安装 shadcn 依赖**

```bash
npm install lucide-react class-variance-authority clsx tailwind-merge
```

**Step 2: 运行 shadcn init**

```bash
npx shadcn@latest init
```

交互式问答选择：
- Style: `Default`
- Base color: `Slate`
- CSS variables: `Yes`

Expected: 生成 `components.json`，更新 `src/index.css` 注入 CSS 变量，创建 `src/lib/utils.ts`

**Step 3: 安装所有需要的 shadcn 组件**

```bash
npx shadcn@latest add button input select badge card separator scroll-area checkbox tabs
```

Expected: `src/components/ui/` 目录下生成对应组件文件，无报错

**Step 4: 验证安装**

```bash
npm run dev
```

Expected: 页面正常加载，无 TS/模块错误

**Step 5: Commit**

```bash
git add components.json src/index.css src/lib/ src/components/ui/ package.json package-lock.json
git commit -m "chore: init shadcn/ui with button, input, select, badge, card, tabs, scroll-area, checkbox"
```

---

## Task 3: 完善 CSS 变量（亮色 + 暗色）与全局样式

**Files:**
- Modify: `src/index.css`

**Step 1: 在 shadcn 生成的 CSS 基础上，追加自定义 CSS 变量**

在 `src/index.css` 的 `@layer base` 中追加以下自定义变量（在 shadcn 已生成内容之后）：

```css
@layer base {
  :root {
    --sidebar-background: 0 0% 100%;
    --sidebar-border: 220 13% 91%;
  }

  .dark {
    --sidebar-background: 222 84% 5%;
    --sidebar-border: 215 28% 17%;
  }

  * {
    @apply border-border;
  }

  body {
    @apply bg-background text-foreground;
    font-feature-settings: "rlig" 1, "calt" 1;
  }
}

@keyframes pulse-ring {
  0% { box-shadow: 0 0 0 0 rgba(239, 68, 68, 0.4); }
  70% { box-shadow: 0 0 0 14px rgba(239, 68, 68, 0); }
  100% { box-shadow: 0 0 0 0 rgba(239, 68, 68, 0); }
}

.recording-pulse {
  animation: pulse-ring 1.5s ease-out infinite;
}
```

**Step 2: 验证**

```bash
npm run dev
```

Expected: 页面背景和文字颜色使用 CSS 变量，无报错

**Step 3: Commit**

```bash
git add src/index.css
git commit -m "style: add sidebar CSS variables and recording pulse animation"
```

---

## Task 4: 重构 App.tsx — 侧边栏双栏 Layout Shell

**Files:**
- Modify: `src/App.tsx`

**Step 1: 重写 `src/App.tsx`**

```tsx
import React from "react";
import { BrowserRouter, Routes, Route } from "react-router-dom";
import { Sidebar } from "./components/Sidebar";
import { Home } from "./pages/Home";
import { Meeting } from "./pages/Meeting";
import { Settings } from "./pages/Settings";

export default function App() {
  return (
    <BrowserRouter>
      <div className="flex h-screen bg-background overflow-hidden">
        <Sidebar />
        <main className="flex-1 overflow-auto">
          <Routes>
            <Route path="/" element={<Home />} />
            <Route path="/meeting/:id" element={<Meeting />} />
            <Route path="/settings" element={<Settings />} />
          </Routes>
        </main>
      </div>
    </BrowserRouter>
  );
}
```

**Step 2: 验证类型检查通过（Sidebar 尚未存在，先跳过 dev 验证）**

编写完 Task 5（Sidebar）后再验证。

---

## Task 5: 新建 Sidebar.tsx 组件

**Files:**
- Create: `src/components/Sidebar.tsx`

**Step 1: 创建 `src/components/Sidebar.tsx`**

```tsx
import React, { useEffect, useState } from "react";
import { useNavigate, useLocation } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import { Mic, Settings, Plus } from "lucide-react";
import { ScrollArea } from "./ui/scroll-area";
import { Button } from "./ui/button";
import { Separator } from "./ui/separator";
import { useMeetingStore } from "../store/meetingStore";
import { cn } from "../lib/utils";
import type { Meeting } from "../types";
import { formatDateTime } from "../utils/format";

const statusDot: Record<Meeting["status"], string> = {
  idle: "bg-muted-foreground/40",
  recording: "bg-destructive animate-pulse",
  processing: "bg-amber-500 animate-pulse",
  completed: "bg-emerald-500",
  error: "bg-destructive",
};

export function Sidebar() {
  const navigate = useNavigate();
  const location = useLocation();
  const { meetings, setMeetings, setError } = useMeetingStore();
  const [newTitle, setNewTitle] = useState("");
  const [creating, setCreating] = useState(false);

  useEffect(() => {
    invoke<Meeting[]>("list_meetings")
      .then(setMeetings)
      .catch((e) => setError(String(e)));
  }, []);

  async function createMeeting() {
    const title = newTitle.trim() || `会议 ${new Date().toLocaleString("zh-CN")}`;
    try {
      setCreating(true);
      const meeting = await invoke<Meeting>("create_meeting", { title });
      setMeetings([meeting, ...meetings]);
      setNewTitle("");
      navigate(`/meeting/${meeting.id}`);
    } catch (e) {
      setError(String(e));
    } finally {
      setCreating(false);
    }
  }

  function currentMeetingId(): number | null {
    const match = location.pathname.match(/\/meeting\/(\d+)/);
    return match ? parseInt(match[1]) : null;
  }

  const activeMeetingId = currentMeetingId();

  return (
    <aside className="w-60 shrink-0 flex flex-col border-r border-border bg-[hsl(var(--sidebar-background))] h-full">
      {/* Logo */}
      <div className="flex items-center gap-2 px-4 py-4">
        <div className="flex h-7 w-7 items-center justify-center rounded-lg bg-primary text-primary-foreground">
          <Mic className="h-4 w-4" />
        </div>
        <div>
          <p className="text-sm font-semibold leading-none">Memo AI</p>
          <p className="text-[10px] text-muted-foreground leading-none mt-0.5">AI 会议助手</p>
        </div>
      </div>

      <Separator />

      {/* New meeting input */}
      <div className="px-3 py-2 flex gap-1.5">
        <input
          type="text"
          value={newTitle}
          onChange={(e) => setNewTitle(e.target.value)}
          onKeyDown={(e) => e.key === "Enter" && createMeeting()}
          placeholder="会议标题…"
          className="flex-1 min-w-0 text-xs px-2 py-1.5 rounded-md border border-input bg-background placeholder:text-muted-foreground focus:outline-none focus:ring-1 focus:ring-ring"
        />
        <Button
          size="icon"
          variant="outline"
          className="h-7 w-7 shrink-0"
          onClick={createMeeting}
          disabled={creating}
          title="新建会议"
        >
          <Plus className="h-3.5 w-3.5" />
        </Button>
      </div>

      {/* Meeting list */}
      <ScrollArea className="flex-1 px-2">
        <div className="space-y-0.5 py-1">
          {meetings.length === 0 ? (
            <p className="text-xs text-muted-foreground text-center py-6">暂无会议记录</p>
          ) : (
            meetings.map((m) => (
              <button
                key={m.id}
                onClick={() => navigate(`/meeting/${m.id}`)}
                className={cn(
                  "w-full text-left rounded-md px-2 py-2 text-xs transition-colors",
                  "hover:bg-accent hover:text-accent-foreground",
                  activeMeetingId === m.id
                    ? "bg-accent text-accent-foreground font-medium"
                    : "text-foreground/80"
                )}
              >
                <div className="flex items-center gap-1.5 mb-0.5">
                  <span
                    className={cn("h-1.5 w-1.5 rounded-full shrink-0", statusDot[m.status])}
                  />
                  <span className="truncate font-medium">{m.title}</span>
                </div>
                <p className="text-[10px] text-muted-foreground pl-3">
                  {formatDateTime(m.start_time)}
                </p>
              </button>
            ))
          )}
        </div>
      </ScrollArea>

      <Separator />

      {/* Settings */}
      <div className="px-2 py-2">
        <button
          onClick={() => navigate("/settings")}
          className={cn(
            "w-full flex items-center gap-2 rounded-md px-2 py-2 text-xs transition-colors",
            "hover:bg-accent hover:text-accent-foreground",
            location.pathname === "/settings"
              ? "bg-accent text-accent-foreground font-medium"
              : "text-muted-foreground"
          )}
        >
          <Settings className="h-3.5 w-3.5" />
          设置
        </button>
      </div>
    </aside>
  );
}
```

**Step 2: 验证开发服务器正常**

```bash
npm run dev
```

Expected: 侧边栏出现在左侧，会议列表可正常加载，点击"+"可创建会议

**Step 3: Commit**

```bash
git add src/App.tsx src/components/Sidebar.tsx
git commit -m "feat: add sidebar layout shell with meeting list and new meeting input"
```

---

## Task 6: 重写 Home.tsx — 欢迎/空状态页

**Files:**
- Modify: `src/pages/Home.tsx`

**Step 1: 重写 `src/pages/Home.tsx`**

```tsx
import React from "react";
import { Mic } from "lucide-react";

export function Home() {
  return (
    <div className="flex h-full flex-col items-center justify-center gap-4 text-center px-8">
      <div className="flex h-16 w-16 items-center justify-center rounded-2xl bg-primary/10">
        <Mic className="h-8 w-8 text-primary" />
      </div>
      <div>
        <h2 className="text-xl font-semibold text-foreground">开始一次会议</h2>
        <p className="mt-1.5 text-sm text-muted-foreground max-w-xs">
          在左侧输入会议标题并按 Enter，或点击 + 按钮创建新会议
        </p>
      </div>
    </div>
  );
}
```

**Step 2: Commit**

```bash
git add src/pages/Home.tsx
git commit -m "feat: rewrite Home page as welcome empty state"
```

---

## Task 7: 重写 MeetingCard.tsx — 侧边栏已无需此组件

**说明：** 侧边栏已在 `Sidebar.tsx` 内联实现了会议列表项。`MeetingCard.tsx` 已不再被任何页面使用。将其保留但清空为简化版，以免删除导致 import 报错（若其他地方有引用）。

**Files:**
- Modify: `src/components/MeetingCard.tsx`

**Step 1: 检查 MeetingCard 是否还有其他引用**

用 Grep 搜索 `MeetingCard` 在除 `MeetingCard.tsx` 自身外是否有引用：

```bash
grep -r "MeetingCard" src/ --include="*.tsx" --include="*.ts"
```

**Step 2a: 若无其他引用（预期如此），直接删除**

由于 `Home.tsx` 已不使用 `MeetingCard`，删除该文件：

```bash
rm src/components/MeetingCard.tsx
```

**Step 2b: 若有引用，重写为 Tailwind 版本并保留**

按实际情况处理。

**Step 3: Commit**

```bash
git add src/components/MeetingCard.tsx
git commit -m "refactor: remove unused MeetingCard component (replaced by Sidebar inline items)"
```

---

## Task 8: 重写 RecordButton.tsx

**Files:**
- Modify: `src/components/RecordButton.tsx`

**Step 1: 重写 `src/components/RecordButton.tsx`**

```tsx
import React from "react";
import { Mic, Square } from "lucide-react";
import { cn } from "../lib/utils";

interface RecordButtonProps {
  isRecording: boolean;
  disabled?: boolean;
  onStart: () => void;
  onStop: () => void;
}

export function RecordButton({ isRecording, disabled, onStart, onStop }: RecordButtonProps) {
  return (
    <button
      onClick={isRecording ? onStop : onStart}
      disabled={disabled}
      aria-label={isRecording ? "停止录音" : "开始录音"}
      className={cn(
        "flex h-20 w-20 items-center justify-center rounded-full transition-all duration-200 focus:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2",
        isRecording
          ? "bg-destructive text-destructive-foreground recording-pulse"
          : "bg-primary text-primary-foreground shadow-lg hover:bg-primary/90 hover:shadow-xl active:scale-95",
        disabled && "cursor-not-allowed bg-muted text-muted-foreground shadow-none recording-pulse-none opacity-60"
      )}
    >
      {isRecording ? (
        <Square className="h-7 w-7 fill-current" />
      ) : (
        <Mic className="h-7 w-7" />
      )}
    </button>
  );
}
```

**Step 2: 验证**

```bash
npm run dev
```

导航到一个会议页面，确认录音按钮渲染正确，点击时出现脉冲动画。

**Step 3: Commit**

```bash
git add src/components/RecordButton.tsx
git commit -m "feat: rewrite RecordButton with Tailwind and pulse animation"
```

---

## Task 9: 重写 TranscriptView.tsx

**Files:**
- Modify: `src/components/TranscriptView.tsx`

**Step 1: 重写 `src/components/TranscriptView.tsx`**

```tsx
import React, { useEffect, useRef } from "react";
import { ScrollArea } from "./ui/scroll-area";
import type { Transcript } from "../types";
import { formatTimestamp } from "../utils/format";

interface TranscriptViewProps {
  transcripts: Transcript[];
}

export function TranscriptView({ transcripts }: TranscriptViewProps) {
  const bottomRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [transcripts]);

  if (transcripts.length === 0) {
    return (
      <div className="flex items-center justify-center py-12 text-sm text-muted-foreground">
        暂无转写内容
      </div>
    );
  }

  return (
    <ScrollArea className="h-[400px] pr-3">
      <div className="flex flex-col gap-3">
        {transcripts.map((t) => (
          <div key={t.id} className="flex gap-3">
            <span className="mt-0.5 shrink-0 text-[11px] tabular-nums text-muted-foreground w-10">
              {formatTimestamp(t.timestamp)}
            </span>
            <div className="text-sm leading-relaxed">
              {t.speaker && (
                <span className="font-semibold text-primary mr-1">{t.speaker}：</span>
              )}
              <span className="text-foreground">{t.text}</span>
            </div>
          </div>
        ))}
        <div ref={bottomRef} />
      </div>
    </ScrollArea>
  );
}
```

**Step 2: Commit**

```bash
git add src/components/TranscriptView.tsx
git commit -m "feat: rewrite TranscriptView with shadcn ScrollArea"
```

---

## Task 10: 重写 ActionItemList.tsx

**Files:**
- Modify: `src/components/ActionItemList.tsx`

**Step 1: 重写 `src/components/ActionItemList.tsx`**

```tsx
import React from "react";
import { Checkbox } from "./ui/checkbox";
import { cn } from "../lib/utils";
import type { ActionItem } from "../types";

interface ActionItemListProps {
  items: ActionItem[];
  onToggle?: (id: number, status: "pending" | "done") => void;
}

export function ActionItemList({ items, onToggle }: ActionItemListProps) {
  if (items.length === 0) {
    return (
      <div className="flex items-center justify-center py-8 text-sm text-muted-foreground">
        暂无行动项
      </div>
    );
  }

  return (
    <ul className="flex flex-col gap-2">
      {items.map((item) => (
        <li
          key={item.id}
          className={cn(
            "flex items-start gap-3 rounded-lg border px-3 py-2.5 transition-colors",
            item.status === "done"
              ? "border-emerald-200 bg-emerald-50/50 dark:border-emerald-800 dark:bg-emerald-950/30"
              : "border-border bg-card"
          )}
        >
          <Checkbox
            id={`action-${item.id}`}
            checked={item.status === "done"}
            onCheckedChange={() =>
              onToggle?.(item.id, item.status === "done" ? "pending" : "done")
            }
            className="mt-0.5 shrink-0"
          />
          <div className="flex-1 min-w-0">
            <label
              htmlFor={`action-${item.id}`}
              className={cn(
                "block text-sm font-medium cursor-pointer",
                item.status === "done"
                  ? "line-through text-muted-foreground"
                  : "text-foreground"
              )}
            >
              {item.task}
            </label>
            {(item.owner || item.deadline) && (
              <div className="mt-1 flex gap-3 text-[11px] text-muted-foreground">
                {item.owner && <span>负责人：{item.owner}</span>}
                {item.deadline && <span>截止：{item.deadline}</span>}
              </div>
            )}
          </div>
        </li>
      ))}
    </ul>
  );
}
```

**Step 2: Commit**

```bash
git add src/components/ActionItemList.tsx
git commit -m "feat: rewrite ActionItemList with shadcn Checkbox"
```

---

## Task 11: 重写 Meeting.tsx — Tabs 布局

**Files:**
- Modify: `src/pages/Meeting.tsx`

**Step 1: 重写 `src/pages/Meeting.tsx`**

```tsx
import React, { useEffect } from "react";
import { useParams } from "react-router-dom";
import { invoke } from "@tauri-apps/api/core";
import { Badge } from "../components/ui/badge";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "../components/ui/tabs";
import { RecordButton } from "../components/RecordButton";
import { TranscriptView } from "../components/TranscriptView";
import { ActionItemList } from "../components/ActionItemList";
import { useMeetingStore } from "../store/meetingStore";
import { useRecording } from "../hooks/useRecording";
import type { Meeting as MeetingType, Transcript, ActionItem } from "../types";

const statusBadge: Record<MeetingType["status"], { label: string; variant: "default" | "destructive" | "secondary" | "outline" }> = {
  idle: { label: "待录音", variant: "secondary" },
  recording: { label: "录音中", variant: "destructive" },
  processing: { label: "AI 处理中", variant: "outline" },
  completed: { label: "已完成", variant: "default" },
  error: { label: "出错", variant: "destructive" },
};

export function Meeting() {
  const { id } = useParams<{ id: string }>();
  const meetingId = id ? parseInt(id) : null;

  const {
    currentMeeting,
    setCurrentMeeting,
    transcripts,
    setTranscripts,
    actionItems,
    setActionItems,
    setCurrentMeetingStatus,
  } = useMeetingStore();

  const { isRecording, error, startRecording, stopRecording } = useRecording(meetingId);

  useEffect(() => {
    if (!meetingId) return;
    loadMeeting();
    loadTranscripts();
    loadActionItems();
  }, [meetingId]);

  async function loadMeeting() {
    const meeting = await invoke<MeetingType>("get_meeting", { id: meetingId });
    setCurrentMeeting(meeting);
  }

  async function loadTranscripts() {
    const data = await invoke<Transcript[]>("get_transcripts", { meetingId });
    setTranscripts(data);
  }

  async function loadActionItems() {
    const data = await invoke<ActionItem[]>("get_action_items", { meetingId });
    setActionItems(data);
  }

  async function handleStopAndProcess() {
    const audioPath = await stopRecording();
    if (!audioPath || !meetingId) return;
    await invoke("transcribe_audio", { audioPath, meetingId });
    await loadTranscripts();
    setCurrentMeetingStatus("processing");
    try {
      await invoke("run_pipeline", { meetingId });
      setCurrentMeetingStatus("completed");
      await loadMeeting();
      await loadActionItems();
    } catch {
      setCurrentMeetingStatus("error");
    }
  }

  async function handleToggleActionItem(itemId: number, status: "pending" | "done") {
    await invoke("update_action_item_status", { id: itemId, status });
    await loadActionItems();
  }

  if (!currentMeeting) {
    return (
      <div className="flex h-full items-center justify-center text-sm text-muted-foreground">
        加载中…
      </div>
    );
  }

  const badgeConfig = statusBadge[currentMeeting.status];

  return (
    <div className="flex flex-col h-full">
      {/* Header */}
      <div className="flex items-center justify-between border-b border-border px-6 py-4 shrink-0">
        <h2 className="text-lg font-semibold text-foreground truncate pr-4">
          {currentMeeting.title}
        </h2>
        <Badge variant={badgeConfig.variant} className="shrink-0">
          {badgeConfig.label}
        </Badge>
      </div>

      {/* Record area */}
      <div className="flex flex-col items-center gap-3 py-6 border-b border-border shrink-0">
        <RecordButton
          isRecording={isRecording}
          disabled={currentMeeting.status === "processing"}
          onStart={startRecording}
          onStop={handleStopAndProcess}
        />
        {currentMeeting.status === "processing" && (
          <p className="text-sm text-amber-500 font-medium">AI 正在处理会议内容…</p>
        )}
        {error && (
          <p className="text-sm text-destructive">{error}</p>
        )}
      </div>

      {/* Tabs */}
      <Tabs defaultValue="transcript" className="flex-1 flex flex-col overflow-hidden px-6 pt-4">
        <TabsList className="shrink-0">
          <TabsTrigger value="transcript">转写</TabsTrigger>
          <TabsTrigger value="actions">行动项</TabsTrigger>
          <TabsTrigger value="summary">总结</TabsTrigger>
          <TabsTrigger value="report">报告</TabsTrigger>
        </TabsList>

        <TabsContent value="transcript" className="flex-1 overflow-auto mt-4">
          <TranscriptView transcripts={transcripts} />
        </TabsContent>

        <TabsContent value="actions" className="flex-1 overflow-auto mt-4">
          <ActionItemList items={actionItems} onToggle={handleToggleActionItem} />
        </TabsContent>

        <TabsContent value="summary" className="flex-1 overflow-auto mt-4">
          {currentMeeting.summary ? (
            <div className="rounded-xl border border-border bg-card p-4 text-sm leading-relaxed whitespace-pre-wrap text-foreground">
              {currentMeeting.summary}
            </div>
          ) : (
            <div className="flex items-center justify-center py-12 text-sm text-muted-foreground">
              暂无总结，完成录音后 AI 将自动生成
            </div>
          )}
        </TabsContent>

        <TabsContent value="report" className="flex-1 overflow-auto mt-4">
          {currentMeeting.report ? (
            <div className="rounded-xl border border-border bg-card p-4 text-sm leading-relaxed whitespace-pre-wrap font-mono text-foreground">
              {currentMeeting.report}
            </div>
          ) : (
            <div className="flex items-center justify-center py-12 text-sm text-muted-foreground">
              暂无报告，完成录音后 AI 将自动生成
            </div>
          )}
        </TabsContent>
      </Tabs>
    </div>
  );
}
```

**Step 2: 验证**

```bash
npm run dev
```

打开任意会议页，确认：Tabs 正常切换，RecordButton 居中显示，Badge 状态正确

**Step 3: Commit**

```bash
git add src/pages/Meeting.tsx
git commit -m "feat: rewrite Meeting page with Tabs layout and shadcn components"
```

---

## Task 12: 重写 Settings.tsx

**Files:**
- Modify: `src/pages/Settings.tsx`

**Step 1: 重写 `src/pages/Settings.tsx`**

```tsx
import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Button } from "../components/ui/button";
import { Input } from "../components/ui/input";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "../components/ui/select";
import { Card, CardContent, CardHeader, CardTitle } from "../components/ui/card";
import { Separator } from "../components/ui/separator";
import { useSettingsStore } from "../store/settingsStore";
import type { AppSettings } from "../types";
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
  }, []);

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
          <CardTitle className="text-sm font-semibold text-muted-foreground uppercase tracking-wide">
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
                setLocal({ ...local, llm_provider: { ...local.llm_provider, type: v as "ollama" | "openai" } })
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
                  setLocal({ ...local, llm_provider: { ...local.llm_provider, api_key: e.target.value || null } })
                }
                placeholder="sk-..."
              />
            </div>
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader className="pb-3">
          <CardTitle className="text-sm font-semibold text-muted-foreground uppercase tracking-wide">
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
```

**Step 2: 验证**

```bash
npm run dev
```

打开设置页，确认：Card 分组样式正确，Select 可正常展开，保存按钮状态切换正常

**Step 3: Commit**

```bash
git add src/pages/Settings.tsx
git commit -m "feat: rewrite Settings page with shadcn Card, Input, Select"
```

---

## Task 13: 最终验证 + TypeScript 类型检查

**Step 1: 全量类型检查**

```bash
npx tsc --noEmit
```

Expected: 无 error（允许有 warning）

**Step 2: 全量功能验证（手动）**

```bash
npm run dev
```

验证清单：
- [ ] 侧边栏常驻，Logo 和导航渲染正确
- [ ] 新建会议：输入标题回车 → 跳转到会议页
- [ ] 会议列表：侧边栏显示所有会议，当前会议高亮
- [ ] Meeting 页：RecordButton、Badge、Tabs 渲染正确
- [ ] Settings 页：Card 分组、Select 可操作、保存反馈
- [ ] 暗色模式：系统切换暗色后页面颜色跟随变化

**Step 3: 最终 Commit**

```bash
git add -A
git commit -m "feat: complete frontend migration to Tailwind CSS v4 + shadcn/ui with sidebar layout"
```
