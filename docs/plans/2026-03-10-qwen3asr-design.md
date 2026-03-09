# Qwen3-ASR 集成设计文档

**日期**：2026-03-10
**状态**：已确认，待实施

## 背景

Qwen3-ASR 是阿里巴巴 Qwen 团队发布的本地语音识别模型，通过 Docker 容器 + vLLM 提供兼容 OpenAI 的 HTTP API。相比现有的 Whisper（本地 CLI）和阿里云 ASR（云端），Qwen3-ASR 提供本地推理 + 高精度多语言识别能力。

## 验证结论

经过实测验证：

- **base64 方案可行**：WAV 文件 base64 编码后通过 `data:audio/wav;base64,` 前缀发送，API 正常响应
- **响应格式固定**：`language XX<asr_text>实际文本`，模型忽略所有自定义指令
- **无原生时间戳**：模型不输出时间戳，需通过音频分块近似
- **中英文均支持**：自动语言检测，无需指定

## 架构设计

```
WAV 文件
  ↓ 按 30s 切片（hound 库）
[chunk_0][chunk_1]...[chunk_n]
  ↓ 顺序 HTTP POST（base64 编码）
Qwen3-ASR API（http://localhost:8000/v1/chat/completions）
  ↓ 解析 "language XX<asr_text>文本"
TranscriptSegment { start, end, text }
  ↓ 汇总
Vec<TranscriptSegment>
```

## 改动清单

### Rust 后端

| 文件 | 类型 | 说明 |
|------|------|------|
| `src-tauri/src/asr/qwen3asr.rs` | 新建 | `Qwen3AsrProvider` 实现 `AsrProvider` trait |
| `src-tauri/src/asr/mod.rs` | 修改 | 新增 `pub mod qwen3asr` |
| `src-tauri/src/asr/provider.rs` | 修改 | `build_asr` 增加 `"qwen3_asr"` 分支 |
| `src-tauri/src/commands.rs` | 修改 | `AppConfig` 增加 `qwen3_asr_url` 字段 |

### 前端

| 文件 | 类型 | 说明 |
|------|------|------|
| `src/types/index.ts` | 修改 | `AsrProviderType` 增加 `"qwen3_asr"` |
| `src/pages/Settings.tsx` | 修改 | 新增 Qwen3-ASR 选项 + URL 配置面板 |
| `src/i18n/locales/zh.ts` | 修改 | 新增中文文案 |
| `src/i18n/locales/en.ts` | 修改 | 新增英文文案 |

## 核心实现：qwen3asr.rs

```rust
pub struct Qwen3AsrProvider {
    api_url: String,       // 默认 "http://localhost:8000"
    chunk_secs: u32,       // 默认 30
}
```

### 分块算法

1. 用 `hound` 读取 WAV 文件头（采样率、声道数、位深）
2. 按 `chunk_secs * sample_rate` 帧切片
3. 每片写入临时内存 WAV（保留完整 WAV header）
4. base64 编码后 POST 到 API
5. 解析响应文本，构造 `TranscriptSegment { start, end, text }`
6. 空文本段跳过，单块失败记录警告后继续

### 时间戳策略

```
start = chunk_index * chunk_secs（秒）
end   = min((chunk_index + 1) * chunk_secs, 音频总时长)
```

### 文本解析

```rust
let text = if let Some(pos) = raw.find("<asr_text>") {
    raw[pos + "<asr_text>".len()..].trim().to_string()
} else {
    raw.trim().to_string()
};
```

## AppConfig 新增字段

```rust
#[serde(default = "default_qwen3_asr_url")]
pub qwen3_asr_url: String,

fn default_qwen3_asr_url() -> String {
    "http://localhost:8000".into()
}
```

## 前端 Settings 面板

```
ASR 引擎选择器：新增 "Qwen3-ASR（本地）" 选项
当选中 qwen3_asr 时展示：
  - API 地址输入框（默认 http://localhost:8000）
  - 提示文案：需先启动 Docker 容器
```

## 不支持的能力

| 能力 | 说明 |
|------|------|
| 实时字幕 | 批量 HTTP API，不支持流式音频输入，无法实现 `StreamingAsrSession` |
| 词级时间戳 | 模型固定输出纯文本，时间戳为 30s 粒度近似值 |

## 超时策略

- 单块请求超时：60 秒
- 无总超时限制（由块数决定，长录音自动适应）

## 依赖确认

- `hound`：WAV 文件读写（已在 Cargo.toml 中）
- `reqwest` blocking：HTTP 请求（已在 Cargo.toml 中）
- `base64`：编码（已在 Cargo.toml 中）
