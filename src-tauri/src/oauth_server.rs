use std::sync::OnceLock;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter};
use tauri_plugin_opener::OpenerExt;
use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use url::Url;
use crate::oauth;
use base64::{engine::general_purpose, Engine as _};
use rand::{rng, RngCore};

/// 使用 OnceLock 代替 lazy_static 存储 OAuth 流程中的临时数据
static PENDING_LOGIN: OnceLock<Mutex<Option<PendingLogin>>> = OnceLock::new();

fn get_pending_login() -> &'static Mutex<Option<PendingLogin>> {
    PENDING_LOGIN.get_or_init(|| Mutex::new(None))
}

struct PendingLogin {
    pkce: oauth::PkceCodes,
    state: String,
    port: u16,
}

/// 生成与官方一致的 state (Base64 编码的32字节随机数)
fn generate_state() -> String {
    let mut bytes = [0u8; 32];
    rng().fill_bytes(&mut bytes);
    general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

/// 官方固定端口
const DEFAULT_PORT: u16 = 1455;

/// 准备 OAuth 流程并返回授权 URL
#[tauri::command]
pub async fn start_oauth_login(app_handle: AppHandle) -> Result<String, String> {
    // 1. 强制使用固定端口 1455 (先 kill 占用进程)
    let _ = std::process::Command::new("sh")
        .arg("-c")
        .arg(format!("lsof -ti:{} | xargs kill -9 2>/dev/null", DEFAULT_PORT))
        .output();
    
    // 等待端口释放
    tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
    
    let listener = TcpListener::bind(format!("127.0.0.1:{}", DEFAULT_PORT)).await
        .map_err(|e| format!("无法绑定本地端口 {}: {}", DEFAULT_PORT, e))?;
    let port = DEFAULT_PORT;
    
    // 2. 生成 PKCE 和 State (与官方一致)
    let pkce = oauth::generate_pkce();
    let state = generate_state();
    let redirect_uri = format!("http://localhost:{}/auth/callback", port);
    
    // 3. 构造授权 URL (与官方完全一致: 手动拼接, 不对特殊字符编码)
    let qs = format!(
        "response_type=code&client_id={}&redirect_uri={}&scope={}&code_challenge={}&code_challenge_method=S256&id_token_add_organizations=true&codex_cli_simplified_flow=true&state={}&originator=codex_vscode",
        oauth::CLIENT_ID,
        redirect_uri,
        "openid profile email offline_access",
        pkce.code_challenge,
        state
    );
    
    let auth_url = format!("{}?{}", oauth::AUTH_URL, qs);
    
    // 4. 保存状态，开启监听任务
    {
        let mut pending = get_pending_login().lock().unwrap();
        *pending = Some(PendingLogin { 
            pkce: pkce.clone(), 
            state: state.clone(),
            port 
        });
    }
    
    // 5. 启动异步监听
    let app_handle_clone = app_handle.clone();
    tokio::spawn(async move {
        handle_callback(listener, app_handle_clone, state).await;
    });
    
    // 6. 打开浏览器
    let _ = app_handle.opener().open_url(&auth_url, None::<String>);
    
    Ok(auth_url)
}

/// 监听回调
async fn handle_callback(listener: TcpListener, app_handle: AppHandle, expected_state: String) {
    if let Ok((mut socket, _)) = listener.accept().await {
        let mut buffer = [0; 4096];
        if let Ok(n) = socket.read(&mut buffer).await {
            let request = String::from_utf8_lossy(&buffer[..n]);
            
            // 解析 URL
            let first_line = request.lines().next().unwrap_or("");
            let parts: Vec<&str> = first_line.split_whitespace().collect();
            
            if parts.len() > 1 {
                let callback_url = format!("http://localhost{}", parts[1]);
                if let Ok(url) = Url::parse(&callback_url) {
                    let params: std::collections::HashMap<_, _> = url.query_pairs().into_owned().collect();
                    
                    let code = params.get("code");
                    let state = params.get("state");
                    
                    if let (Some(c), Some(s)) = (code, state) {
                        if s == &expected_state {
                            // 发送成功 HTML 并通知前端
                            let response = "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\n\r\n\
                                <html><body><h1>授权成功</h1><p>已成功连接 OpenAI，你可以关闭此窗口并回到应用。</p>\
                                <script>setTimeout(() => window.close(), 3000)</script></body></html>";
                            let _ = socket.write_all(response.as_bytes()).await;
                            
                            // 存储 Code 供后续 finalize_oauth_login 调用
                            app_handle.emit("oauth-callback-received", c).unwrap();
                            return;
                        }
                    }
                }
            }
        }
        
        let response = "HTTP/1.1 400 Bad Request\r\n\r\n授权失败: State 校验不通过或参数缺失";
        let _ = socket.write_all(response.as_bytes()).await;
    }
}

/// 最后一步：使用捕获到的 Code 交换 Token (由前端触发)
#[tauri::command]
pub async fn complete_oauth_login(code: String) -> Result<oauth::TokenResponse, String> {
    // 提取所需数据并立即释放锁，避免跨 await 持有 MutexGuard
    let (code_verifier, port) = {
        let mut pending_lock = get_pending_login().lock().map_err(|_| "锁被污染")?;
        let pending = pending_lock.take().ok_or("登录流程已过期或未启动")?;
        (pending.pkce.code_verifier, pending.port)
    };
    
    let redirect_uri = format!("http://localhost:{}/auth/callback", port);
    
    oauth::exchange_code(&code, &redirect_uri, &code_verifier).await
}
