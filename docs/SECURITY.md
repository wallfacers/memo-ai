# 安全策略

## 核心承诺

**"你的会议内容，只有你自己看到。"**

这是 memo-ai 对用户的核心隐私承诺，所有安全决策围绕此展开。

## 数据本地化

- 所有会议数据（音频、转写、摘要）仅存储在用户本机
- 默认使用本地 LLM（Ollama），数据不离开设备
- 使用 OpenAI API 时，必须明确告知用户数据将发送到 OpenAI 服务器

## API Key 安全

- OpenAI API Key 存储在操作系统 Keychain（不存入 SQLite 明文）
- 使用 Tauri 的安全存储 API：`tauri-plugin-stronghold` 或系统 keychain
- 前端从不直接持有 API Key，仅 Rust 层访问

## Tauri 安全配置

`tauri.conf.json` 中的 CSP（内容安全策略）必须严格配置：

```json
{
  "security": {
    "csp": "default-src 'self'; connect-src 'self' http://localhost:11434"
  }
}
```

- 只允许访问 `localhost:11434`（Ollama 默认端口）
- 禁止加载外部脚本和资源
- OpenAI API 调用从 Rust 层发出，不经过前端

## 权限最小化

Tauri 插件权限按需申请，不申请未使用的权限：
- `shell`：仅用于打开外部链接（非命令执行）
- 文件系统：仅访问 `app_data_dir` 目录

## 网络访问

| 目标 | 协议 | 用途 | 默认 |
|------|------|------|------|
| localhost:11434 | HTTP | Ollama API | 启用 |
| api.openai.com | HTTPS | OpenAI API | 用户启用 |
| 其他所有地址 | - | - | 禁止 |

## 录音权限

- macOS：应用必须声明麦克风使用目的（`NSMicrophoneUsageDescription`）
- Windows：通过 WASAPI 请求录音权限，失败时给出友好提示

## 已知风险与缓解

| 风险 | 缓解措施 |
|------|---------|
| SQLite 文件被其他进程读取 | 文件权限设置为仅当前用户可读 |
| 录音文件未加密 | v1.0 计划支持可选的本地加密 |
| Ollama 本地服务被局域网访问 | 提示用户确认 Ollama 绑定到 127.0.0.1 |
