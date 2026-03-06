# 前端开发规范

## 技术约定

- TypeScript 严格模式（`"strict": true`）
- 函数组件 + Hooks，不使用 Class 组件
- 状态管理：Zustand（`src/store/`）
- 路由：React Router v6
- 样式：Tailwind CSS + shadcn/ui（`shadcn@latest`）
- UI 组件库：shadcn/ui，**禁止手动复制组件代码，必须通过 CLI 命令安装**

## 目录结构约定

```
src/
├── components/     # 纯 UI 组件，不直接调用 Tauri API
├── pages/          # 页面级组件，可调用 hooks
├── hooks/          # 业务 hooks，封装 Tauri 调用和副作用
├── store/          # Zustand store
├── utils/          # 纯函数工具
└── types/          # 共享类型，从这里导入，不在组件内定义
```

## shadcn/ui 使用规范

shadcn/ui 组件**必须通过 CLI 命令安装**，不允许手动创建或复制组件文件。

```bash
# 初始化（首次）
npx shadcn@latest init

# 安装组件（必须用此方式）
npx shadcn@latest add button
npx shadcn@latest add dialog
npx shadcn@latest add input
# ... 以此类推
```

安装后的组件位于 `src/components/ui/`，**不要手动修改该目录下的文件**。
业务定制在包装组件中完成，不直接改 shadcn 组件源码。

```
src/components/
├── ui/              # shadcn 安装的原始组件（只读，不手动修改）
│   ├── button.tsx
│   ├── dialog.tsx
│   └── ...
└── RecordButton.tsx # 业务组件，可在此封装 shadcn 组件
```

## 组件规范

- 组件文件名使用 PascalCase：`MeetingCard.tsx`
- 每个组件只做一件事，超过 200 行考虑拆分
- Props 类型定义在组件文件顶部，命名为 `{ComponentName}Props`

## Tauri API 调用规范

所有 Tauri `invoke()` 调用必须通过 `src/hooks/useTauriCommands.ts` 封装，组件不直接调用 `invoke()`。

```typescript
// 正确
const { startRecording } = useTauriCommands()
await startRecording(meetingId)

// 错误
await invoke('start_recording', { meetingId })
```

## Store 规范

Store 按领域拆分：
- `meetingStore`：会议列表、当前会议状态
- `settingsStore`：用户配置（LLM 选择、模型参数等）

Store 中不放 UI 状态（弹窗开关等），UI 状态用组件本地 `useState`。

## 错误处理

- Tauri 调用使用 try/catch，catch 中更新 store 中的 error 字段
- 不在组件中直接 console.error，通过 store 传递错误状态给 UI

## 格式化工具

时间格式化使用 `dayjs`（已安装），不用 `Date` 原生 API 直接格式化。

```typescript
import dayjs from 'dayjs'
dayjs(meeting.start_time).format('YYYY-MM-DD HH:mm')
```
