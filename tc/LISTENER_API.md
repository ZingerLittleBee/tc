# 监听配置 API 文档

本文档描述了 auxm 服务器新增的动态网络监听配置 API 接口。

## 概述

新的监听配置 API 允许用户通过 HTTP 请求动态配置服务器的监听设置，包括：
- 添加/移除监听 IP 地址
- 添加/移除监听端口
- 查询当前监听配置

所有配置更改会自动同步到 eBPF 程序，实现实时的网络监控配置更新。

## API 端点

### 1. 获取当前监听配置

**GET** `/api/listeners`

获取当前的监听配置信息。

**响应示例:**
```json
{
  "success": true,
  "data": {
    "listen_ips": ["192.168.1.100", "10.0.0.1"],
    "listen_ports": [8080, 9090],
    "interface": "eth0"
  },
  "error": null,
  "timestamp": "2024-01-01T12:00:00Z"
}
```

### 2. 添加监听 IP 地址

**POST** `/api/listeners/ip`

添加新的监听 IP 地址到监控列表。

**请求体:**
```json
{
  "ip": "192.168.1.200"
}
```

**响应示例:**
```json
{
  "success": true,
  "data": {
    "success": true,
    "message": "成功添加监听 IP 地址: 192.168.1.200",
    "affected_item": "192.168.1.200"
  },
  "error": null,
  "timestamp": "2024-01-01T12:00:00Z"
}
```

### 3. 移除监听 IP 地址

**POST** `/api/listeners/ip/remove`

从监控列表中移除指定的 IP 地址。

**请求体:**
```json
{
  "ip": "192.168.1.200"
}
```

**响应示例:**
```json
{
  "success": true,
  "data": {
    "success": true,
    "message": "成功移除监听 IP 地址: 192.168.1.200",
    "affected_item": "192.168.1.200"
  },
  "error": null,
  "timestamp": "2024-01-01T12:00:00Z"
}
```

### 4. 添加监听端口

**POST** `/api/listeners/port`

添加新的监听端口到监控列表。

**请求体:**
```json
{
  "port": 9090
}
```

**响应示例:**
```json
{
  "success": true,
  "data": {
    "success": true,
    "message": "成功添加监听端口: 9090",
    "affected_item": "9090"
  },
  "error": null,
  "timestamp": "2024-01-01T12:00:00Z"
}
```

### 5. 移除监听端口

**POST** `/api/listeners/port/remove`

从监控列表中移除指定的端口。

**请求体:**
```json
{
  "port": 9090
}
```

**响应示例:**
```json
{
  "success": true,
  "data": {
    "success": true,
    "message": "成功移除监听端口: 9090",
    "affected_item": "9090"
  },
  "error": null,
  "timestamp": "2024-01-01T12:00:00Z"
}
```

## 输入验证

### IP 地址验证
- 必须是有效的 IPv4 地址格式
- 例如: `192.168.1.100`, `10.0.0.1`
- 无效示例: `invalid.ip`, `256.256.256.256`

### 端口验证
- 端口范围: 1-65535
- 端口 0 不被允许
- 必须是有效的整数

## 错误处理

当请求失败时，API 会返回相应的错误信息：

**错误响应示例:**
```json
{
  "success": false,
  "data": null,
  "error": "无效的 IP 地址格式: invalid.ip.address",
  "timestamp": "2024-01-01T12:00:00Z"
}
```

**常见错误:**
- `无效的 IP 地址格式: {ip}` - IP 地址格式不正确
- `端口号不能为 0` - 端口号为 0
- `IP 地址 {ip} 已经在监听列表中` - 重复添加 IP
- `端口 {port} 已经在监听列表中` - 重复添加端口
- `IP 地址 {ip} 不在监听列表中` - 移除不存在的 IP
- `端口 {port} 不在监听列表中` - 移除不存在的端口

## 使用示例

### 使用 curl

```bash
# 获取当前配置
curl http://localhost:8080/api/listeners

# 添加监听 IP
curl -X POST http://localhost:8080/api/listeners/ip \
  -H "Content-Type: application/json" \
  -d '{"ip": "192.168.1.200"}'

# 添加监听端口
curl -X POST http://localhost:8080/api/listeners/port \
  -H "Content-Type: application/json" \
  -d '{"port": 9090}'

# 移除监听 IP
curl -X POST http://localhost:8080/api/listeners/ip/remove \
  -H "Content-Type: application/json" \
  -d '{"ip": "192.168.1.200"}'

# 移除监听端口
curl -X POST http://localhost:8080/api/listeners/port/remove \
  -H "Content-Type: application/json" \
  -d '{"port": 9090}'
```

### 使用 JavaScript (fetch)

```javascript
// 获取当前配置
const config = await fetch('http://localhost:8080/api/listeners')
  .then(res => res.json());

// 添加监听 IP
const addIpResult = await fetch('http://localhost:8080/api/listeners/ip', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({ ip: '192.168.1.200' })
}).then(res => res.json());

// 添加监听端口
const addPortResult = await fetch('http://localhost:8080/api/listeners/port', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({ port: 9090 })
}).then(res => res.json());
```

## 测试

项目包含了完整的测试套件：

### 自动化测试脚本
```bash
# 运行测试脚本（需要服务器运行在 localhost:8080）
./test_listener_api.sh
```

### Rust 集成测试
```bash
# 运行 Rust 测试（需要服务器运行）
cargo test --test listener_api_test -- --ignored
```

## 注意事项

1. **权限要求**: 服务器需要以 root 权限运行以访问 eBPF 功能
2. **实时同步**: 配置更改会在下一个监控周期（约5秒）内同步到 eBPF 程序
3. **持久化**: 当前配置更改不会持久化到磁盘，重启服务器后会恢复到初始配置
4. **并发安全**: API 使用 RwLock 确保并发访问的安全性

## 架构集成

新的监听配置功能与现有系统完全集成：

- **eBPF 同步**: 配置更改自动同步到内核空间的 eBPF 程序
- **Web API**: 使用相同的 Axum 框架和响应格式
- **错误处理**: 统一的错误处理和日志记录
- **状态管理**: 与现有的 AppState 结构集成
