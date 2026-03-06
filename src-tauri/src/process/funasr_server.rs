use std::process::{Child, Command};

pub struct FunAsrServer {
    child: Option<Child>,
    pub ws_url: String,
}

impl FunAsrServer {
    /// 根据配置决定使用外部服务还是启动本地进程
    /// - ws_url 非空 → 直接使用外部服务（不管理进程）
    /// - ws_url 为空 → 检测本地 funasr-server 并尝试启动
    pub fn start(ws_url: &str, server_path: &str, port: u16) -> Result<Self, String> {
        // 1. 有配置 URL → 直接用外部服务
        if !ws_url.is_empty() {
            return Ok(FunAsrServer {
                child: None,
                ws_url: ws_url.to_string(),
            });
        }

        // 2. 无配置 URL → 检测本地 funasr-server 并尝试启动
        let exe = if server_path.is_empty() { "funasr-server" } else { server_path };

        // 先检测可执行文件是否存在
        let check = Command::new(exe).arg("--help").output();
        if check.is_err() {
            return Err(format!(
                "FunASR server not found at '{}'. Install via: pip install funasr-runtime",
                exe
            ));
        }

        let child = Command::new(exe)
            .arg("--port")
            .arg(port.to_string())
            .arg("--model")
            .arg("paraformer-zh")
            .spawn()
            .map_err(|e| format!("Failed to start funasr-server: {}", e))?;

        // 等待服务就绪（简单 sleep 3s）
        std::thread::sleep(std::time::Duration::from_secs(3));

        Ok(FunAsrServer {
            child: Some(child),
            ws_url: format!("ws://localhost:{}", port),
        })
    }

    pub fn stop(&mut self) {
        if let Some(ref mut child) = self.child {
            let _ = child.kill();
        }
        self.child = None;
    }

    pub fn is_managed(&self) -> bool {
        self.child.is_some()
    }
}

impl Drop for FunAsrServer {
    fn drop(&mut self) {
        self.stop();
    }
}

/// 检测本地 funasr-server 是否可用，返回版本信息或错误信息
pub fn check_funasr_server(server_path: &str) -> Result<String, String> {
    let exe = if server_path.is_empty() { "funasr-server" } else { server_path };
    match Command::new(exe).arg("--version").output() {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);
            let version = stdout.lines().chain(stderr.lines())
                .next()
                .unwrap_or("unknown")
                .trim()
                .to_string();
            Ok(if version.is_empty() {
                "funasr-server found".to_string()
            } else {
                version
            })
        }
        Err(_) => Err(format!(
            "funasr-server not found at '{}'. Install via: pip install funasr-runtime",
            exe
        )),
    }
}
