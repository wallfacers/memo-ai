# 产品规格：会议采集系统

## 功能目标

采集用户会议的音频输入，支持本地麦克风和系统音（用于在线会议场景）。

## 用户场景

1. **本地会议**：多人在同一房间，录制麦克风
2. **在线会议**：Zoom / Teams / 腾讯会议，同时录制麦克风 + 系统音
3. **单人语音备忘**：录制个人发言，快速生成备忘录

## 核心功能

### 音频来源

| 来源 | 说明 | 优先级 |
|------|------|--------|
| 麦克风 | 物理麦克风设备 | MVP |
| 系统音 | 扬声器/虚拟设备回环 | MVP |
| 双通道 | 麦克风 + 系统音同时录制 | MVP |

### 录音控制

- 开始录音：点击录音按钮，状态变为 `recording`
- 停止录音：点击停止，触发 AI Pipeline 处理，状态变为 `processing`
- 暂停/继续：v1.0 支持
- 实时波形显示：v1.0 支持

### 音频质量

- 采样率：16kHz（Whisper 最优）
- 格式：WAV（录制中）→ MP3/Opus（存储）
- 降噪：软件降噪（v1.0）

## 平台实现

### Windows（MVP）

使用 WASAPI（Windows Audio Session API）：
- `src-tauri/src/audio/wasapi.rs`
- 支持原生系统音回环采集（Loopback Device）

### macOS（v1.0）

使用 CoreAudio：
- 麦克风：原生支持
- 系统音：需用户安装虚拟音频设备（BlackHole 2ch）
- 在 Settings 页提供安装引导

## 数据存储

录音文件存储在 Tauri 应用数据目录：

```
{app_data_dir}/recordings/{meeting_id}.wav
```

路径记录在 `meetings.audio_path` 字段。

## 状态机

```
idle → recording → processing → done
                └──────────────────► error
```
