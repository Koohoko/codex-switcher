//! 后台调度器 - Token 自动刷新
//!
//! 借鉴 CodexBar 的全局定时轮询 + Antigravity 的静默刷新

use crate::account::AccountStore;
use std::sync::{Arc, Mutex};
use tokio::time::{interval, Duration};
use tauri::Emitter;

/// 启动后台 Token 刷新调度器
pub fn start(store: Arc<Mutex<AccountStore>>, app_handle: tauri::AppHandle) {
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
                        match refresh_token_silently(refresh_token, &account.auth_json).await {
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
                
                // 发送事件通知前端更新账号列表
                let _ = app_handle.emit("accounts-updated", ());
            } else {
                println!("[Scheduler] 所有 Token 状态良好，无需刷新");
            }
        }
    });
}

/// 检查 Token 是否即将过期（剩余 < 10 分钟）
fn is_token_expiring_soon(auth_json: &serde_json::Value) -> bool {
    // 优先从 tokens.expires_at 获取 (可能是 RFC3339 字符串)
    let expires_at_val = auth_json.get("tokens")
        .and_then(|t| t.get("expires_at"))
        .or_else(|| auth_json.get("expires_at"));

    if let Some(val) = expires_at_val {
        let timestamp = if let Some(ts) = val.as_i64() {
            ts
        } else if let Some(iso_str) = val.as_str() {
            // 解析 RFC3339 字符串
            chrono::DateTime::parse_from_rfc3339(iso_str)
                .map(|dt| dt.timestamp())
                .unwrap_or(0)
        } else {
            0
        };

        if timestamp > 0 {
            let now = chrono::Utc::now().timestamp();
            let remaining = timestamp - now;
            return remaining < 600; // 10 分钟
        }
    }
    false
}

/// 静默刷新 Token
async fn refresh_token_silently(refresh_token: &str, old_auth: &serde_json::Value) -> Result<serde_json::Value, String> {
    // 复用 OAuth 模块的刷新逻辑
    let token_response = crate::oauth::refresh_access_token(refresh_token).await?;
    
    // 计算新的过期时间
    let expires_in = token_response.expires_in.unwrap_or(3600);
    let expires_at_iso = (chrono::Utc::now() + chrono::Duration::seconds(expires_in as i64)).to_rfc3339();
    
    // 构建新的 auth_json，保留原有结构
    let mut new_auth = old_auth.clone();
    if let Some(obj) = new_auth.as_object_mut() {
        obj.insert("last_refresh".to_string(), serde_json::json!(chrono::Utc::now().to_rfc3339()));
        
        if let Some(tokens_obj) = obj.get_mut("tokens").and_then(|v| v.as_object_mut()) {
            tokens_obj.insert("access_token".to_string(), serde_json::json!(token_response.access_token));
            if let Some(rt) = token_response.refresh_token {
                tokens_obj.insert("refresh_token".to_string(), serde_json::json!(rt));
            }
            if let Some(it) = token_response.id_token {
                tokens_obj.insert("id_token".to_string(), serde_json::json!(it));
            }
            tokens_obj.insert("expires_at".to_string(), serde_json::json!(expires_at_iso));
        } else {
            // 如果旧结构受损，尝试重建基础结构
            obj.insert("tokens".to_string(), serde_json::json!({
                "access_token": token_response.access_token,
                "refresh_token": token_response.refresh_token,
                "id_token": token_response.id_token,
                "expires_at": expires_at_iso
            }));
        }
    }
    
    Ok(new_auth)
}
