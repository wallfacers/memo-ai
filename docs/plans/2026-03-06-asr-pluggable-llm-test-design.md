# 设计文档：ASR 可插拔后端 + LLM 连接测试

**日期：** 2026-03-06
**状态：** 已批准，待执行
**执行计划：** [docs/exec-plans/active/04-asr-pluggable-llm-test.md](../exec-plans/active/04-asr-pluggable-llm-test.md)

---

## 目标

1. 将 ASR 层重构为可插拔 trait 架构，支持本地 Whisper CLI 和阿里云 ASR 作为可切换后端
2. Settings 页为本地 Whisper 新增检测向导，为阿里云 ASR 提供配置面板
3. Settings 页 LLM 配置区新增「测试连接」功能，验证 Ollama/OpenAI 配置是否可用

---

## Section 1：ASR 可插拔架构

### Rust 后端目录结构

```
src-tauri/src/asr/
├── mod.rs          — 导出 AsrProvider trait 和工厂函数 build_asr()
├── provider.rs     — AsrProvider trait 定义
├── whisper.rs      — WhisperAsr（实现 AsrProvider trait）
├── aliyun.rs       — AliyunAsr（新增，实现 AsrProvider trait）
└── transcript.rs   — TranscriptSegment（不变）
```

### AsrProvider trait

```rust
pub trait AsrProvider: Send {
    fn transcribe(&self, audio_path: &Path) -> AppResult<Vec<TranscriptSegment>>;
    fn name(&self) -> &'static str;
}
```

### 工厂函数

```rust
pub fn build_asr(config: &AppConfig) -> Box<dyn AsrProvider>
```

根据 `config.asr_provider` 字段返回对应实现。

### 阿里云 ASR 调用方式

上传音频文件 → 调用「录音文件识别」REST API → 轮询结果（异步转同步封装在 Rust 层）。

---

## Section 2：LLM 连接测试

### Tauri 命令

```rust
#[derive(Serialize)]
pub struct LlmTestResult {
    pub success: bool,
    pub message: String,    // 成功："连接正常 (234ms)"；失败：错误详情
    pub latency_ms: u64,
}

#[tauri::command]
pub fn test_llm_connection(settings: AppConfig, ...) -> Result<LlmTestResult, String>
```

- 使用**传入的表单值**（非已保存配置），支持保存前先验证
- Ollama：`POST /api/generate`，超时 10s，区分「连接拒绝」vs「模型不存在」
- OpenAI：`POST /v1/chat/completions`，区分「401 Key 错误」vs「网络错误」

### 前端状态

```typescript
type TestStatus = "idle" | "testing" | "ok" | "fail";
```

UI：`[测试连接]  ● 连接正常 (234ms)`，切换 Provider 时重置为 `idle`。

---

## Section 3：Settings UI

### ASR 面板

顶部 Provider 选择器（本地 Whisper / 阿里云 ASR），切换后面板内容动态替换。

**本地 Whisper 面板**
- `whisper-cli 路径` + `[检测]` 按钮（调用 `check_whisper_cli` 命令）
- 检测结果：找到 → 绿点 + 版本号；未找到 → 红点 + 下载链接
- `模型文件目录` + `识别语言`

**阿里云 ASR 面板**
- `AppKey`、`AccessKey ID`、`AccessKey Secret`（可显示/隐藏）
- 控制台链接说明
- `[测试配置]` 按钮，验证鉴权有效性

---

## Section 4：配置变更

### AppConfig 扩展（settings.json，向后兼容）

```rust
#[derive(Serialize, Deserialize)]
pub struct AppConfig {
    pub llm_provider: LlmProviderConfig,
    #[serde(default = "default_asr_provider")]
    pub asr_provider: String,              // "local_whisper" | "aliyun"
    pub whisper_cli_path: String,
    pub whisper_model_dir: String,
    pub whisper_model: String,
    pub language: String,
    #[serde(default)]
    pub aliyun_asr_app_key: String,
    #[serde(default)]
    pub aliyun_asr_access_key_id: String,
    #[serde(default)]
    pub aliyun_asr_access_key_secret: String,
}
```

旧配置文件通过 `#[serde(default)]` 自动兼容，无需数据库迁移。

### Cargo 新增依赖

```toml
reqwest = { version = "0.12", features = ["json", "multipart"] }
base64 = "0.22"
```

### 前端类型

```typescript
type AsrProviderType = "local_whisper" | "aliyun";

interface AppSettings {
  llm_provider: LlmProvider;
  asr_provider: AsrProviderType;
  whisper_cli_path: string;
  whisper_model_dir: string;
  whisper_model: string;
  language: string;
  aliyun_asr_app_key: string;
  aliyun_asr_access_key_id: string;
  aliyun_asr_access_key_secret: string;
}
```

---

## 不在本次范围内

- 其他云 ASR 厂商（腾讯云、Azure 等）——架构支持，实现留后续
- 实时流式转写——依赖 WebSocket，留 v1.0 后续
- ASR 结果缓存——留技术债跟踪
