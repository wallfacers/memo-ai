# 前端全量迁移设计文档

**日期：** 2026-03-06
**范围：** 全量重写前端样式，从内联 style 迁移至 Tailwind CSS + shadcn/ui，并重构布局为侧边栏双栏结构

---

## 背景

当前前端所有样式均使用内联 `style={{...}}`，未遵循 FRONTEND.md 规定的 Tailwind CSS + shadcn/ui 技术栈。视觉上较为朴素，缺乏桌面应用质感。

## 目标

1. 完全迁移至 Tailwind CSS，消除所有内联样式
2. 引入 shadcn/ui 组件库（通过 CLI 安装）
3. 支持亮色（默认）+ 系统跟随暗色（`darkMode: 'media'`）
4. 重构为侧边栏双栏布局

---

## 整体架构

### App Shell 布局

```
┌─────────────────────────────────────────────────────┐
│  侧边栏 240px    │  主内容区（flex-1）                  │
│  ─────────────  │  ──────────────────────────────── │
│  🎙 Memo AI     │                                    │
│                 │  [Meeting 页 / Settings 页 / Home]  │
│  ▶ 会议列表      │                                    │
│    会议标题 1    │                                    │
│    会议标题 2    │                                    │
│    ...          │                                    │
│                 │                                    │
│  [+ 新会议]      │                                    │
│                 │                                    │
│  ⚙ 设置         │                                    │
└─────────────────────────────────────────────────────┘
```

### 路由结构

- `/` — 欢迎/空状态页（右侧内容区）
- `/meeting/:id` — 会议详情页（右侧），侧边栏高亮当前会议
- `/settings` — 设置页（右侧）

侧边栏（`Sidebar.tsx`）为新建组件，常驻于所有路由。

---

## 视觉设计系统

### 色板（CSS 变量，亮/暗双模式）

| Token | 亮色 | 暗色 | 用途 |
|-------|------|------|------|
| `--primary` | blue-600 | blue-500 | 主操作、品牌色 |
| `--destructive` | red-500 | red-400 | 录音中、错误 |
| `--success` | emerald-500 | emerald-400 | 完成状态 |
| `--warning` | amber-500 | amber-400 | AI 处理中 |
| `--background` | gray-50 | gray-950 | 页面底色 |
| `--sidebar-bg` | white | gray-900 | 侧边栏背景 |
| `--card` | white | gray-800 | 卡片背景 |
| `--muted` | gray-500 | gray-400 | 次要文字 |
| `--border` | gray-200 | gray-700 | 边框 |

### 关键组件规格

**RecordButton**
- 待机：蓝色实心圆，80px，Mic lucide icon
- 录音中：红色 + CSS keyframes 脉冲光晕动画
- 禁用：gray-300，`cursor-not-allowed`

**MeetingCard（侧边栏版）**
- 左侧 2px 状态色条 + 标题 + 时间 + Badge
- 选中：`bg-primary/10 border-l-primary`
- 悬停：`hover:bg-muted/50`

**状态 Badge 颜色**

| 状态 | Badge 颜色 |
|------|-----------|
| idle | gray |
| recording | red + animate-pulse |
| processing | amber |
| completed | emerald |
| error | red |

**Meeting 页布局（Tabs）**

```
会议标题 + 状态 Badge
RecordButton 居中
─────────────────────────────
[转写] [行动项] [总结] [报告]  ← shadcn Tabs
Tab 内容区
```

---

## 文件变更清单

| 文件 | 操作 |
|------|------|
| `tailwind.config.js` | 新建 |
| `src/index.css` | 重写，引入 Tailwind + CSS 变量 |
| `src/App.tsx` | 重构为双栏 layout |
| `src/components/Sidebar.tsx` | 新建 |
| `src/components/RecordButton.tsx` | 重写 |
| `src/components/MeetingCard.tsx` | 重写（侧边栏紧凑版） |
| `src/components/TranscriptView.tsx` | 重写 |
| `src/components/ActionItemList.tsx` | 重写 |
| `src/pages/Home.tsx` | 重写（欢迎/空状态） |
| `src/pages/Meeting.tsx` | 重写（Tabs 布局） |
| `src/pages/Settings.tsx` | 重写（shadcn Input/Select） |

## shadcn/ui 组件安装顺序

```bash
npx shadcn@latest init
npx shadcn@latest add button input select badge card separator scroll-area checkbox tabs
```

---

## 约束

- shadcn 组件通过 CLI 安装，不手动创建 `src/components/ui/` 下的文件
- 不修改 `src/hooks/`、`src/store/`、`src/utils/`、`src/types/` 中的逻辑代码
- 所有样式只使用 Tailwind class，禁止新增内联 style
