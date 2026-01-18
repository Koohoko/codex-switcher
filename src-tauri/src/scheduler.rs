//! 后台调度器 - Token 自动刷新
//!
//! 借鉴 CodexBar 的全局定时轮询 + Antigravity 的静默刷新

use crate::account::AccountStore;
use std::sync::{Arc, Mutex};
use tokio::time::{interval, Duration};

/// 启动后台 Token 刷新调度器
pub fn start(store: Arc<Mutex<AccountStore>>) {
    // 使用 Tauri 的 async runtime 而不是直接 tokio::spawn
    // 因为在 setup() 中调用时 Tokio runtime 可能尚未完全初始化
    tauri::async_runtime::spawn(async move {
        // 默认 30 分钟轮询一次
        let mut ticker = interval(Duration::from_secs(30 * 60));
        
        println!("✅ 后台调度器已启动 (间隔: 30 分钟)");
        
        loop {
            ticker.tick().await;
            
            println!("[Scheduler] 开始检查 Token 有效期...");
            
            // 获取所有账号
            let accounts = {
                let store = store.lock().unwrap();
                store.accounts.values().cloned().collect::<Vec<_>>()
            };
            
            let mut refreshed_count = 0;
            
            for account in accounts {
                // 检查 Token 是否即将过期
                if let Some(ref refresh_token) = account.refresh_token {
                    if is_token_expiring_soon(&account.auth_json) {
                        println!("[Scheduler] 账号 {} Token 即将过期，正在刷新...", account.name);
                        
                        // 调用刷新逻辑
                        match refresh_token_silently(refresh_token).await {
                            Ok(new_auth) => {
                                // 更新 store 中的 auth_json
                                let mut store = store.lock().unwrap();
                                if let Some(acc) = store.accounts.get_mut(&account.id) {
                                    acc.auth_json = new_auth;
                                    refreshed_count += 1;
                                    println!("[Scheduler] ✅ 账号 {} Token 刷新成功", account.name);
                                }
                                let _ = store.save();
                            }
                            Err(e) => {
                                println!("[Scheduler] ❌ 账号 {} Token 刷新失败: {}", account.name, e);
                            }
                        }
                    }
                }
            }
            
            if refreshed_count > 0 {
                println!("[Scheduler] 本轮刷新了 {} 个账号的 Token", refreshed_count);
            } else {
                println!("[Scheduler] 所有 Token 状态良好，无需刷新");
            }
        }
    });
}

/// 检查 Token 是否即将过期（剩余 < 10 分钟）
fn is_token_expiring_soon(auth_json: &serde_json::Value) -> bool {
    if let Some(expires_at) = auth_json.get("expires_at").and_then(|v| v.as_i64()) {
        let now = chrono::Utc::now().timestamp();
        let remaining = expires_at - now;
        return remaining < 600; // 10 分钟 = 600 秒
    }
    false
}

/// 静默刷新 Token
async fn refresh_token_silently(refresh_token: &str) -> Result<serde_json::Value, String> {
    // 复用 OAuth 模块的刷新逻辑
    let token_response = crate::oauth::refresh_access_token(refresh_token).await?;
    
    // 构建新的 auth_json
    let now = chrono::Utc::now().timestamp();
    let expires_in = token_response.expires_in.unwrap_or(3600); // 默认 1 小时
    let expires_at = now + expires_in as i64;
    
    Ok(serde_json::json!({
        "access_token": token_response.access_token,
        "refresh_token": token_response.refresh_token,
        "expires_at": expires_at,
        "expires_in": expires_in
    }))
}
