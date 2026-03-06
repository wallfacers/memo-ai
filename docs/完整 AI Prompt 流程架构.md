
目标是把会议录音 → 转写 → 结构化 → 总结 → 报告 的 **AI处理流程完全标准化**，方便直接接入 LLM（如 OpenAI API 或 Ollama）。

核心思想是：
**把一次会议拆成多个 AI Agent Prompt 阶段。**

---

# 一、完整 AI Prompt 流程架构

整个流程建议拆成 **6个AI步骤**

```
会议录音
 ↓
语音转文字
 ↓
①文本清洗
 ↓
②说话人整理
 ↓
③结构化提取
 ↓
④会议总结
 ↓
⑤行动项提取
 ↓
⑥报告生成
```

这样做的好处：

* LLM更稳定
* 输出结构化
* 可控性强

---

# 二、Prompt Stage 1：会议文本清洗

输入：
ASR转写文本（可能有错字、断句问题）

目标：

```
修复语句
补充标点
保持原意
```

Prompt：

```
你是一名会议记录整理助手。

任务：
对会议语音转写文本进行清洗，使其更易阅读。

要求：
1 保留原始含义
2 修复断句
3 添加标点
4 删除明显的语音识别错误
5 不要总结

输出格式：
仅输出整理后的会议文本

会议文本：
{{transcript}}
```

输出示例：

```
张三：我们这次主要讨论AI项目上线计划。

李四：我建议第一阶段先进行内部测试，然后再逐步开放给客户。
```

---

# 三、Prompt Stage 2：说话人整理

如果 ASR 已有 Speaker Diarization，可以进一步整理。

目标：

```
统一说话人
合并连续发言
```

Prompt：

```
你是一名会议结构整理助手。

任务：
整理会议对话结构。

要求：
1 合并同一说话人的连续发言
2 保持时间顺序
3 输出清晰对话格式

输出格式：

[说话人]：
内容
```

输出：

```
张三：
我们这次主要讨论AI项目上线计划。

李四：
我建议第一阶段先进行内部测试。
```

---

# 四、Prompt Stage 3：会议结构化提取

目标：

```
从会议中提取核心信息
```

结构：

```
会议主题
参会人
讨论要点
决议
风险
```

Prompt：

```
你是一名企业会议分析助手。

任务：
从会议内容中提取结构化信息。

输出JSON格式：

{
"topic":"",
"participants":[],
"key_points":[],
"decisions":[],
"risks":[]
}

会议内容：
{{meeting_text}}
```

示例输出：

```
{
"topic":"AI项目上线计划",
"participants":["张三","李四"],
"key_points":[
"讨论AI项目上线时间",
"讨论内部测试阶段"
],
"decisions":[
"第一阶段进行内部测试"
],
"risks":[
"模型稳定性风险"
]
}
```

---

# 五、Prompt Stage 4：会议总结

目标：

生成 **会议纪要**

Prompt：

```
你是一名企业会议秘书。

任务：
根据会议内容生成专业会议纪要。

要求：

包含以下部分：

1 会议主题
2 会议时间
3 参会人员
4 讨论内容
5 会议结论

风格：
简洁、正式、适合企业文档。

会议内容：
{{meeting_text}}
```

输出示例：

```
会议主题：AI项目上线计划

参会人员：
张三、李四

讨论内容：
讨论了AI项目上线阶段规划。

会议结论：
第一阶段进行内部测试。
```

---

# 六、Prompt Stage 5：行动项提取

这个是会议软件 **最重要功能**。

目标：

```
自动生成TODO
```

Prompt：

```
你是一名项目管理助手。

任务：
从会议中提取所有行动项。

要求：

识别：
任务
负责人
截止时间

输出JSON：

[
{
"task":"",
"owner":"",
"deadline":""
}
]

会议内容：
{{meeting_text}}
```

输出示例：

```
[
{
"task":"完成API设计",
"owner":"张三",
"deadline":"2026-03-10"
},
{
"task":"准备测试环境",
"owner":"李四",
"deadline":"2026-03-12"
}
]
```

---

# 七、Prompt Stage 6：报告生成

目标：

生成 **可发送的会议报告**

Prompt：

```
你是一名企业报告生成助手。

任务：
根据会议纪要和行动项生成会议报告。

格式：

# 会议报告

## 会议主题

## 会议总结

## 关键决策

## 行动项

## 风险

输入：

会议纪要：
{{summary}}

行动项：
{{actions}}
```

输出：

```
# 会议报告

## 会议主题
AI项目上线计划

## 会议总结
会议讨论了AI项目上线阶段。

## 关键决策
第一阶段进行内部测试。

## 行动项
张三：完成API设计（3月10日）
李四：准备测试环境（3月12日）
```

---

# 八、AI Agent Pipeline

最终 AI pipeline：

```
ASR
 ↓
Clean Transcript
 ↓
Speaker Organize
 ↓
Structured Extraction
 ↓
Meeting Summary
 ↓
Action Items
 ↓
Report
```

可以写成：

```
AI Meeting Agent Pipeline
```

---

# 九、工程实现流程

真实系统流程：

```
Audio
 ↓
ASR (Whisper)
 ↓
Transcript
 ↓
Prompt 1 Clean
 ↓
Prompt 2 Structure
 ↓
Prompt 3 Extract
 ↓
Prompt 4 Summary
 ↓
Prompt 5 Actions
 ↓
Prompt 6 Report
```

LLM可以是：

* OpenAI API
* Ollama
* LM Studio

---

# 十、提示词系统优化（高级）

真实产品会做 **Prompt模板系统**：

```
prompt/
 ├ clean_prompt.txt
 ├ structure_prompt.txt
 ├ summary_prompt.txt
 ├ action_prompt.txt
 └ report_prompt.txt
```

这样可以：

* 版本管理
* A/B测试
* 不同模型适配

---

# 十一、最重要的Prompt技巧

会议AI一定要：

### 1 使用JSON输出

```
结构化
```

### 2 拆多个Prompt

不要一个Prompt做完。

### 3 使用上下文

例如：

```
会议纪要 + 行动项
```

生成报告。

---

# 十二、未来升级（Agent）

未来可以变成：

```
Meeting Agent
```

支持：

```
问会议
```

例如：

```
谁负责API开发？
```

AI回答。

---
