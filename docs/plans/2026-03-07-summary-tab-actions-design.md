# 总结选项卡操作按钮设计文档

**日期：** 2026-03-07
**状态：** 已确认，待实现

## 需求概述

为会议页面的「总结」选项卡添加操作按钮：复制、手动编辑、重新生成。按钮风格参考 ChatGPT 消息操作按钮（ghost 图标风格）。

## 方案选择

采用**方案 A：新建 `SummaryTab` 组件**，与项目现有模式一致（类比 `TranscriptView`、`ActionItemList`）。

## 组件结构

### 新文件：`src/components/SummaryTab.tsx`

**Props：**
```typescript
interface SummaryTabProps {
  meeting: Meeting;
  onSummaryUpdated: (newSummary: string) => void;
}
```

### UI 布局

```
┌─────────────────────────────────────────────┐
│ [编辑✏] [复制📋] [重新生成↺]   （右对齐）    │
├─────────────────────────────────────────────┤
│                                             │
│  Markdown 渲染 / Textarea 编辑区            │
│                                             │
└─────────────────────────────────────────────┘
```

### 按钮风格（ChatGPT ghost 风格）

```tsx
<Button
  variant="ghost"
  size="icon"
  className="h-7 w-7 rounded-md text-muted-foreground hover:text-foreground"
>
  <Copy className="h-4 w-4" />
</Button>
```

- 透明背景，hover 时浅灰圆角矩形
- 图标颜色 `text-muted-foreground` → hover `text-foreground`
- 尺寸 `h-7 w-7`，`rounded-md`
- Lucide outline 图标：`Pencil`、`Copy`/`Check`、`RefreshCw`/`Loader2`
- 带 `title` tooltip

## 状态管理

```typescript
const [isEditing, setIsEditing] = useState(false);
const [editText, setEditText] = useState(meeting.summary ?? "");
const [isCopied, setIsCopied] = useState(false);
const [isRegenerating, setIsRegenerating] = useState(false);
const debounceTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
```

## 交互细节

### 复制
- 点击后图标从 `Copy` 切换为绿色 `Check`，1.5 秒后恢复

### 内联编辑
- 点"编辑"（`Pencil`）→ Markdown 渲染区变为 `<textarea>`
- 编辑图标变为 `Check`，点击退出编辑模式
- 编辑时"重新生成"按钮禁用
- textarea 高度自适应（`min-h-[200px]`）

### 自动保存（防抖 1 秒）
- `editText` 变化时，清除旧 timer，1 秒后调用 `update_meeting_summary` Tauri 命令
- 组件卸载时立即 flush 未完成的 timer

### 重新生成
- 调用 `regenerate_summary(meetingId)` → 仅重跑 LLM 总结阶段
- 重新生成中：三个按钮全 `disabled`，`RefreshCw` 替换为 `Loader2` 旋转
- 完成后调用 `onSummaryUpdated(newSummary)` 通知父组件

## 边界情况

| 情况 | 处理 |
|------|------|
| 无总结内容 | 复制/编辑 disabled，重新生成可用 |
| 会议处理中（`processing`）| 三个按钮全 disabled |
| 重新生成中 | 三个按钮全 disabled |
| 编辑模式中 | 重新生成 disabled |
| 重新生成失败 | console.error，不引入新 UI 库 |

## 需新增的 Tauri 命令（Rust 侧）

| 命令 | 参数 | 说明 |
|------|------|------|
| `update_meeting_summary` | `id: i64, summary: String` | 写库更新 summary 字段 |
| `regenerate_summary` | `meeting_id: i64` | 仅重跑 LLM 总结阶段，返回新 summary |

## 涉及文件

| 文件 | 变更类型 |
|------|----------|
| `src/components/SummaryTab.tsx` | 新建 |
| `src/pages/Meeting.tsx` | 修改（替换 summary TabsContent，引入 SummaryTab） |
| `src/hooks/useTauriCommands.ts` | 修改（新增两个 hook） |
| `src-tauri/src/commands.rs` | 修改（新增两个命令） |
| `src-tauri/src/db/` | 修改（新增 update_summary DB 操作） |
| `src-tauri/src/llm/pipeline.rs` | 修改（拆出单独的总结阶段函数） |
