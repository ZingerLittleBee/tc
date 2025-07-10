# TC Network Monitor - Swagger API 文档

本项目已集成 Swagger/OpenAPI 文档，提供完整的 API 接口文档和交互式测试界面。

## 🚀 快速开始

### 启动服务器

```bash
# 编译项目
cargo build

# 启动 Web API 服务器
cargo run --bin tc -- --web-only
```

### 访问 Swagger 文档

服务器启动后，您可以通过以下方式访问 API 文档：

- **Swagger UI**: http://localhost:8080/swagger-ui
  - 交互式 API 文档界面
  - 可以直接在浏览器中测试 API 端点
  - 查看请求/响应示例

- **OpenAPI JSON**: http://localhost:8080/api-docs/openapi.json
  - 原始的 OpenAPI 3.0 规范文件
  - 可用于生成客户端代码或导入其他工具

## 📚 API 端点概览

### 仪表板 (Dashboard)
- `GET /api/dashboard` - 获取实时仪表板数据
- `GET /api/status` - 获取系统状态信息

### 流量数据 (Traffic)
- `GET /api/ip/history` - 获取指定 IP 的历史流量数据
- `GET /api/ip/protocols` - 获取指定 IP 的协议统计历史
- `GET /api/ports/top` - 获取热门端口统计

### 监听配置 (Listeners)
- `GET /api/listeners` - 获取当前监听配置
- `POST /api/listeners/ip` - 添加监听 IP 地址
- `POST /api/listeners/ip/remove` - 移除监听 IP 地址
- `POST /api/listeners/port` - 添加监听端口
- `POST /api/listeners/port/remove` - 移除监听端口

### 系统 (System)
- `GET /health` - 健康检查

## 🔧 技术实现

### 使用的库
- **utoipa**: OpenAPI 规范生成
- **utoipa-swagger-ui**: Swagger UI 集成
- **axum**: Web 框架

### 文档结构
```
src/
├── docs.rs              # OpenAPI 规范定义
├── web_api.rs           # API 端点实现和注解
├── analytics.rs         # 数据模型 (带 ToSchema 注解)
├── listener_config.rs   # 配置模型 (带 ToSchema 注解)
└── storage.rs           # 存储模型 (带 ToSchema 注解)
```

### 代码注解示例

```rust
/// 获取实时仪表板数据
#[utoipa::path(
    get,
    path = "/api/dashboard",
    tag = "dashboard"
)]
pub async fn get_dashboard(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<DashboardData>>, StatusCode> {
    // 实现代码...
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct DashboardData {
    pub realtime_metrics: RealtimeMetrics,
    pub top_ips: Vec<IpTrafficSummary>,
    // 其他字段...
}
```

## 🧪 测试

### 自动化测试
运行提供的测试脚本来验证 Swagger 文档：

```bash
./test_swagger.sh
```

测试脚本会：
1. 编译和构建项目
2. 启动服务器
3. 测试所有文档端点
4. 验证 OpenAPI 规范内容
5. 检查关键 API 端点是否已文档化

### 手动测试
1. 启动服务器：`cargo run --bin tc -- --web-only`
2. 打开浏览器访问：http://localhost:8080/swagger-ui
3. 在 Swagger UI 中测试各个 API 端点

## 📝 开发指南

### 添加新的 API 端点

1. **在函数上添加 utoipa 注解**：
```rust
#[utoipa::path(
    get,
    path = "/api/new-endpoint",
    tag = "category"
)]
pub async fn new_endpoint() -> Json<ApiResponse<YourDataType>> {
    // 实现
}
```

2. **为数据模型添加 ToSchema 注解**：
```rust
#[derive(Serialize, Deserialize, ToSchema)]
pub struct YourDataType {
    pub field: String,
}
```

3. **在 docs.rs 中注册端点**：
```rust
#[derive(OpenApi)]
#[openapi(
    // ...
    paths(
        // 现有端点...
        crate::web_api::new_endpoint,
    ),
    components(
        schemas(
            // 现有模式...
            crate::your_module::YourDataType,
        )
    ),
    // ...
)]
pub struct ApiDoc;
```

### 最佳实践

1. **保持文档同步**: 每次修改 API 时都要更新相应的注解
2. **使用有意义的标签**: 将相关的端点分组到同一个标签下
3. **提供示例**: 在可能的情况下为复杂的数据结构提供示例
4. **测试文档**: 定期运行测试脚本确保文档正常工作

## 🔍 故障排除

### 常见问题

1. **编译错误**: 确保所有使用的数据类型都实现了 `ToSchema` trait
2. **端点未显示**: 检查是否在 `docs.rs` 中注册了新的端点
3. **Swagger UI 无法访问**: 确认服务器正在运行且端口 8080 未被占用

### 调试技巧

1. 检查 OpenAPI JSON: 访问 `/api-docs/openapi.json` 查看生成的规范
2. 查看服务器日志: 运行时的错误信息会显示在控制台
3. 使用测试脚本: `./test_swagger.sh` 可以快速验证配置

## 📖 参考资料

- [utoipa 文档](https://docs.rs/utoipa/)
- [OpenAPI 3.0 规范](https://swagger.io/specification/)
- [Swagger UI 文档](https://swagger.io/tools/swagger-ui/)
- [Axum 框架文档](https://docs.rs/axum/)
