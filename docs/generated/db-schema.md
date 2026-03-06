# 数据库 Schema

> 此文件由 `schema/init.sql` 整理生成，请勿手动修改。更新 schema 时同步更新本文件。

数据库：SQLite，文件路径由 Tauri 应用数据目录管理。

## 表结构

### `meetings` — 会议主表

| 字段 | 类型 | 说明 |
|------|------|------|
| id | INTEGER PK | 自增主键 |
| title | TEXT NOT NULL | 会议标题 |
| start_time | TEXT NOT NULL | 开始时间（ISO 8601） |
| end_time | TEXT | 结束时间（ISO 8601） |
| status | TEXT NOT NULL | 状态：idle / recording / processing / done |
| summary | TEXT | AI 生成的会议纪要 |
| report | TEXT | AI 生成的 Markdown 报告 |
| audio_path | TEXT | 录音文件本地路径 |
| created_at | TEXT NOT NULL | 创建时间（ISO 8601） |
| updated_at | TEXT NOT NULL | 更新时间（ISO 8601） |

### `transcripts` — 转写文本

| 字段 | 类型 | 说明 |
|------|------|------|
| id | INTEGER PK | 自增主键 |
| meeting_id | INTEGER FK | 关联 meetings.id（级联删除） |
| speaker | TEXT | 说话人标识（可为 NULL） |
| text | TEXT NOT NULL | 转写文本片段 |
| timestamp | REAL NOT NULL | 片段时间戳（秒） |
| confidence | REAL | ASR 置信度（0.0~1.0） |
| created_at | TEXT NOT NULL | 创建时间 |

索引：`idx_transcripts_meeting_id`

### `action_items` — 行动项

| 字段 | 类型 | 说明 |
|------|------|------|
| id | INTEGER PK | 自增主键 |
| meeting_id | INTEGER FK | 关联 meetings.id（级联删除） |
| task | TEXT NOT NULL | 任务描述 |
| owner | TEXT | 负责人 |
| deadline | TEXT | 截止日期 |
| status | TEXT NOT NULL | 状态：pending / done（默认 pending） |
| created_at | TEXT NOT NULL | 创建时间 |

索引：`idx_action_items_meeting_id`

### `meeting_structures` — 会议结构化数据

| 字段 | 类型 | 说明 |
|------|------|------|
| id | INTEGER PK | 自增主键 |
| meeting_id | INTEGER FK UNIQUE | 关联 meetings.id（1:1，级联删除） |
| topic | TEXT | 会议主题 |
| participants | TEXT NOT NULL | 参会人员 JSON 数组，默认 `[]` |
| key_points | TEXT NOT NULL | 讨论要点 JSON 数组，默认 `[]` |
| decisions | TEXT NOT NULL | 决议 JSON 数组，默认 `[]` |
| risks | TEXT NOT NULL | 风险 JSON 数组，默认 `[]` |
| created_at | TEXT NOT NULL | 创建时间 |

## 关系图

```
meetings (1)
    ├──< transcripts (N)
    ├──< action_items (N)
    └──── meeting_structures (1:1)
```

## 注意事项

- JSON 数组字段（participants、key_points 等）以 TEXT 类型存储，应用层负责序列化/反序列化
- 时间戳统一使用 ISO 8601 字符串（`2026-03-06T10:00:00Z`）
- 级联删除：删除 meeting 会自动删除所有关联记录
