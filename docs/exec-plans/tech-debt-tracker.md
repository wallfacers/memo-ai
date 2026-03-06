# 技术债务追踪

记录已知的技术债务，按优先级排序。每个条目应说明问题、影响和解决方向。

## 格式

| 字段 | 说明 |
|------|------|
| 优先级 | High / Medium / Low |
| 位置 | 文件路径或模块名 |
| 描述 | 问题描述 |
| 影响 | 不解决会怎样 |
| 解决方向 | 建议的修复方案 |

---

## 当前技术债务

_暂无未解决的技术债务。_

---

## 已解决的债务

| 债务 | 解决方案 | 解决于 |
|------|---------|--------|
| Pipeline JSON 解析失败中断 | Stage 3/5 降级返回空值，log::warn 记录 | 02-tech-debt Task 1 |
| Whisper 模型路径硬编码 | AppConfig 新增 whisper_model_dir，Settings 可配置 | 02-tech-debt Task 2 |
| 前端缺少全局错误边界 | ErrorBoundary 组件包裹 App main 区域 | 02-tech-debt Task 3 |
| 数据库无迁移版本管理 | PRAGMA user_version + migrations.rs 版本化迁移 | 02-tech-debt Task 4 |
