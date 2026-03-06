# Prompt 模板参考

所有 Prompt 模板位于项目根目录 `prompts/` 下，共 6 个文件对应 AI Pipeline 的 6 个阶段。

## 文件列表

| 文件 | 阶段 | 输入变量 |
|------|------|---------|
| `01_clean.txt` | 文本清洗 | `{{transcript}}` |
| `02_speaker.txt` | 说话人整理 | `{{transcript}}` |
| `03_structure.txt` | 结构化提取 | `{{meeting_text}}` |
| `04_summary.txt` | 会议总结 | `{{meeting_text}}` |
| `05_actions.txt` | 行动项提取 | `{{meeting_text}}` |
| `06_report.txt` | 报告生成 | `{{summary}}`, `{{actions}}` |

## 修改 Prompt 的注意事项

1. 保留 `{{变量名}}` 占位符，运行时由 `pipeline.rs` 替换
2. JSON 输出的 Prompt（03、05）需保持输出格式的 JSON Schema 不变
3. 修改后在 Settings 页面使用"测试 AI 连接"功能验证效果
4. 建议在修改前备份原文件

## Prompt 设计原则

- 每个 Prompt 只做一件事
- 明确指定输出格式（JSON 或纯文本）
- 避免让模型"猜测"缺失信息，改为返回 null 或空数组
- 中文 Prompt 对中文会议效果更好

## 变量占位符规范

```
{{variable_name}}   # 双花括号，下划线分隔
```

不支持嵌套或条件占位符，复杂逻辑在 Rust 代码中处理。
