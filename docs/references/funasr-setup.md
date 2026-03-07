# FunASR WebSocket 服务器安装指南

**平台：** Windows
**日期：** 2026-03-07

---

## 前提条件

- Python 3.8 ~ 3.11（推荐；3.13 可用但部分依赖可能有兼容性问题）
- pip 已安装
- Git 已安装

---

## 步骤一：安装 funasr Python 包

```powershell
pip install funasr modelscope torch torchaudio
```

> 注意：PyPI 上的 `funasr-runtime`（v0.0.1）是占位包，不提供服务器功能，不要安装它。

---

## 步骤二：克隆 FunASR 仓库（仅取 runtime 脚本）

```powershell
git clone https://github.com/modelscope/FunASR.git --depth=1
```

网络不佳时使用 Gitee 镜像：

```powershell
git clone https://gitee.com/modelscope/FunASR.git --depth=1
```

---

## 步骤三：安装服务端依赖

```powershell
cd FunASR\runtime\python\websocket
pip install -r requirements_server.txt
```

`requirements_server.txt` 内容为 `websockets`。

---

## 步骤四：启动 FunASR WebSocket 服务器

```powershell
python funasr_wss_server.py --certfile "" --host 0.0.0.0 --port 10095 --asr_model paraformer-zh --vad_model fsmn-vad --punc_model ct-punc
```

**参数说明：**

| 参数 | 值 | 说明 |
|------|-----|------|
| `--certfile ""` | 空字符串 | 禁用 TLS，使用 ws:// 明文连接（注意：传 `0` 无效，脚本用字符串长度判断） |
| `--host` | 0.0.0.0 | 监听所有网卡 |
| `--port` | 10095 | 默认端口 |
| `--asr_model` | paraformer-zh | 中文 ASR 模型 |
| `--vad_model` | fsmn-vad | 语音活动检测模型 |
| `--punc_model` | ct-punc | 标点符号恢复模型 |

> 首次启动会自动从 ModelScope 下载模型文件，共约 1GB，需等待。

---

## 步骤五：在 memo-ai 中配置

打开 memo-ai → Settings → ASR Provider → FunASR，填写：

- **WebSocket 地址：** `ws://localhost:10095`
- 其余留默认值

---

## 常见问题

**参数格式**：脚本参数使用下划线（`--asr_model`），不是连字符（`--asr-model`）。

**`funasr.runtime` 模块找不到**：不要用 `python -m funasr.runtime...` 方式启动，直接 `python funasr_wss_server.py` 运行脚本文件。

**ffmpeg 警告**：启动时出现 `ffmpeg is not installed` 提示属正常，服务仍可正常运行（使用 torchaudio 替代）。

**端口冲突**：修改 `--port` 参数，并在 memo-ai Settings 中对应修改 WebSocket 地址。
