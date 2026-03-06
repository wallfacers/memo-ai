# AI 模型参考

## Whisper（ASR）

官方文档：https://github.com/ggerganov/whisper.cpp

### 模型大小对比

| 模型 | 文件大小 | 内存占用 | 相对速度 | 推荐场景 |
|------|---------|---------|---------|---------|
| tiny | 75 MB | ~400 MB | 最快 | 测试/低配设备 |
| base | 142 MB | ~500 MB | 快 | MVP 默认 |
| small | 466 MB | ~1 GB | 中 | 生产推荐 |
| medium | 1.5 GB | ~2 GB | 慢 | 高精度需求 |

### 中文支持

Whisper 原生支持中文，使用 `language = "zh"` 参数指定。

---

## Ollama（本地 LLM）

官方文档：https://ollama.com/

### 推荐模型

| 模型 | 参数量 | 中文能力 | 推荐用途 |
|------|--------|---------|---------|
| llama3.2:3b | 3B | 一般 | 低配设备，快速响应 |
| llama3.1:8b | 8B | 良好 | MVP 默认推荐 |
| qwen2.5:7b | 7B | 优秀 | 中文会议优先推荐 |
| mistral:7b | 7B | 一般 | 英文会议 |

### 启动 Ollama

```bash
# 安装并启动
ollama serve

# 拉取模型
ollama pull llama3.1:8b
ollama pull qwen2.5:7b
```

默认地址：`http://localhost:11434`

---

## OpenAI API（云端 LLM）

### 推荐模型

| 模型 | 说明 | 成本 |
|------|------|------|
| gpt-4o-mini | 默认推荐，性价比最高 | 低 |
| gpt-4o | 最高质量 | 高 |

### API Key 配置

在 Settings 页面输入 API Key，存储在系统 Keychain（不存入数据库明文）。
