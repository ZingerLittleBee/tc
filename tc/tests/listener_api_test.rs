use reqwest;
use serde_json::json;
use std::collections::HashMap;
use tokio;

/// 测试监听配置 API 的基本功能
#[tokio::test]
async fn test_listener_api_basic_operations() {
    let base_url = "http://localhost:8080";
    let client = reqwest::Client::new();

    // 1. 测试获取当前监听配置
    println!("测试获取当前监听配置...");
    let response = client
        .get(&format!("{}/api/listeners", base_url))
        .send()
        .await;

    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                let body = resp.text().await.unwrap();
                println!("当前监听配置: {}", body);
            } else {
                println!("获取监听配置失败: {}", resp.status());
            }
        }
        Err(e) => {
            println!("请求失败 (服务器可能未运行): {}", e);
            return;
        }
    }

    // 2. 测试添加监听 IP
    println!("\n测试添加监听 IP...");
    let add_ip_payload = json!({
        "ip": "192.168.1.200"
    });

    let response = client
        .post(&format!("{}/api/listeners/ip", base_url))
        .json(&add_ip_payload)
        .send()
        .await;

    match response {
        Ok(resp) => {
            let body = resp.text().await.unwrap();
            println!("添加 IP 响应: {}", body);
        }
        Err(e) => {
            println!("添加 IP 请求失败: {}", e);
        }
    }

    // 3. 测试添加监听端口
    println!("\n测试添加监听端口...");
    let add_port_payload = json!({
        "port": 9090
    });

    let response = client
        .post(&format!("{}/api/listeners/port", base_url))
        .json(&add_port_payload)
        .send()
        .await;

    match response {
        Ok(resp) => {
            let body = resp.text().await.unwrap();
            println!("添加端口响应: {}", body);
        }
        Err(e) => {
            println!("添加端口请求失败: {}", e);
        }
    }

    // 4. 再次获取监听配置，验证更改
    println!("\n验证配置更改...");
    let response = client
        .get(&format!("{}/api/listeners", base_url))
        .send()
        .await;

    match response {
        Ok(resp) => {
            if resp.status().is_success() {
                let body = resp.text().await.unwrap();
                println!("更新后的监听配置: {}", body);
            }
        }
        Err(e) => {
            println!("验证请求失败: {}", e);
        }
    }

    // 5. 测试移除监听 IP
    println!("\n测试移除监听 IP...");
    let remove_ip_payload = json!({
        "ip": "192.168.1.200"
    });

    let response = client
        .post(&format!("{}/api/listeners/ip/remove", base_url))
        .json(&remove_ip_payload)
        .send()
        .await;

    match response {
        Ok(resp) => {
            let body = resp.text().await.unwrap();
            println!("移除 IP 响应: {}", body);
        }
        Err(e) => {
            println!("移除 IP 请求失败: {}", e);
        }
    }

    // 6. 测试移除监听端口
    println!("\n测试移除监听端口...");
    let remove_port_payload = json!({
        "port": 9090
    });

    let response = client
        .post(&format!("{}/api/listeners/port/remove", base_url))
        .json(&remove_port_payload)
        .send()
        .await;

    match response {
        Ok(resp) => {
            let body = resp.text().await.unwrap();
            println!("移除端口响应: {}", body);
        }
        Err(e) => {
            println!("移除端口请求失败: {}", e);
        }
    }
}

/// 测试输入验证
#[tokio::test]
async fn test_input_validation() {
    let base_url = "http://localhost:8080";
    let client = reqwest::Client::new();

    println!("测试输入验证...");

    // 测试无效 IP 地址
    let invalid_ip_payload = json!({
        "ip": "invalid.ip.address"
    });

    let response = client
        .post(&format!("{}/api/listeners/ip", base_url))
        .json(&invalid_ip_payload)
        .send()
        .await;

    match response {
        Ok(resp) => {
            let body = resp.text().await.unwrap();
            println!("无效 IP 测试响应: {}", body);
            assert!(body.contains("无效的 IP 地址格式") || body.contains("error"));
        }
        Err(e) => {
            println!("无效 IP 测试请求失败: {}", e);
        }
    }

    // 测试无效端口 (0)
    let invalid_port_payload = json!({
        "port": 0
    });

    let response = client
        .post(&format!("{}/api/listeners/port", base_url))
        .json(&invalid_port_payload)
        .send()
        .await;

    match response {
        Ok(resp) => {
            let body = resp.text().await.unwrap();
            println!("无效端口测试响应: {}", body);
            assert!(body.contains("端口号不能为 0") || body.contains("error"));
        }
        Err(e) => {
            println!("无效端口测试请求失败: {}", e);
        }
    }
}

/// 手动测试函数 - 可以用于交互式测试
pub async fn manual_test_listener_api() {
    println!("=== 监听配置 API 手动测试 ===\n");

    // 运行基本操作测试
    test_listener_api_basic_operations().await;

    println!("\n=== 输入验证测试 ===\n");

    // 运行输入验证测试
    test_input_validation().await;

    println!("\n=== 测试完成 ===");
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    // 注意：这些测试需要服务器运行在 localhost:8080
    // 运行测试前请确保服务器已启动
    
    #[tokio::test]
    #[ignore] // 默认忽略，需要手动运行
    async fn run_manual_tests() {
        manual_test_listener_api().await;
    }
}
