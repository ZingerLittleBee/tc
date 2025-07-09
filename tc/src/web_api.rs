use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;

use crate::analytics::DashboardData;
use crate::listener_config::{
    AddListenerIpRequest, AddListenerPortRequest, ListenerConfig, ListenerConfigResponse,
    ListenerOperationResult, RemoveListenerIpRequest, RemoveListenerPortRequest,
    validate_ip_address, validate_port,
};
use crate::storage::{FlowRecord, PortRecord, ProtocolRecord, TrafficStorage};

// API 响应结构
#[derive(Serialize, Debug)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub timestamp: DateTime<Utc>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            timestamp: Utc::now(),
        }
    }

    pub fn error(error: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
            timestamp: Utc::now(),
        }
    }
}

// 查询参数
#[derive(Deserialize)]
pub struct TimeRangeQuery {
    pub hours: Option<i64>,
    pub start: Option<DateTime<Utc>>,
    pub end: Option<DateTime<Utc>>,
}

#[derive(Deserialize)]
pub struct IpQuery {
    pub ip: String,
    #[serde(flatten)]
    pub time_range: TimeRangeQuery,
}

#[derive(Deserialize)]
pub struct TopPortsQuery {
    pub limit: Option<usize>,
    #[serde(flatten)]
    pub time_range: TimeRangeQuery,
}

// 应用状态
#[derive(Clone)]
pub struct AppState {
    pub storage: Arc<TrafficStorage>,
    pub latest_dashboard_data: Arc<RwLock<Option<DashboardData>>>,
    pub listener_config: Arc<ListenerConfig>,
}

impl AppState {
    pub fn new(storage: TrafficStorage, listener_config: ListenerConfig) -> Self {
        Self {
            storage: Arc::new(storage),
            latest_dashboard_data: Arc::new(RwLock::new(None)),
            listener_config: Arc::new(listener_config),
        }
    }

    pub async fn update_dashboard_data(&self, data: DashboardData) {
        let mut dashboard = self.latest_dashboard_data.write().await;
        *dashboard = Some(data);
    }
}

// API 路由处理器

/// 获取实时仪表板数据
pub async fn get_dashboard(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<DashboardData>>, StatusCode> {
    let dashboard = state.latest_dashboard_data.read().await;

    match dashboard.as_ref() {
        Some(data) => Ok(Json(ApiResponse::success(data.clone()))),
        None => Ok(Json(ApiResponse::error("尚无可用数据".to_string()))),
    }
}

/// 获取指定 IP 的历史流量数据
pub async fn get_ip_history(
    Query(query): Query<IpQuery>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<FlowRecord>>>, StatusCode> {
    let (start_time, end_time) = parse_time_range(&query.time_range);

    // 简单的 IP 地址验证
    if !is_valid_ip(&query.ip) {
        return Ok(Json(ApiResponse::error("无效的 IP 地址格式".to_string())));
    }

    // 将 IP 字符串转换为 u32
    let ip_u32 = match ip_str_to_u32(&query.ip) {
        Ok(ip) => ip,
        Err(e) => return Ok(Json(ApiResponse::error(format!("IP 地址转换错误: {}", e)))),
    };

    match state
        .storage
        .get_ip_flows_history(ip_u32, start_time, end_time)
    {
        Ok(flows) => Ok(Json(ApiResponse::success(flows))),
        Err(e) => {
            eprintln!("获取 IP 历史数据错误: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 获取热门端口统计
pub async fn get_top_ports(
    Query(query): Query<TopPortsQuery>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<PortRecord>>>, StatusCode> {
    let (start_time, end_time) = parse_time_range(&query.time_range);
    let limit = query.limit.unwrap_or(10).min(50); // 最多返回 50 个

    match state.storage.get_top_ports(start_time, end_time, limit) {
        Ok(ports) => Ok(Json(ApiResponse::success(ports))),
        Err(e) => {
            eprintln!("获取热门端口数据错误: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 获取指定 IP 的协议统计历史
pub async fn get_ip_protocols(
    Query(query): Query<IpQuery>,
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<Vec<ProtocolRecord>>>, StatusCode> {
    let (start_time, end_time) = parse_time_range(&query.time_range);

    if !is_valid_ip(&query.ip) {
        return Ok(Json(ApiResponse::error("无效的 IP 地址格式".to_string())));
    }

    let ip_u32 = match ip_str_to_u32(&query.ip) {
        Ok(ip) => ip,
        Err(e) => return Ok(Json(ApiResponse::error(format!("IP 地址转换错误: {}", e)))),
    };

    match state
        .storage
        .get_protocol_stats_history(ip_u32, start_time, end_time)
    {
        Ok(protocols) => Ok(Json(ApiResponse::success(protocols))),
        Err(e) => {
            eprintln!("获取协议统计数据错误: {}", e);
            Err(StatusCode::INTERNAL_SERVER_ERROR)
        }
    }
}

/// 获取系统状态信息
pub async fn get_system_status(State(state): State<AppState>) -> Json<ApiResponse<SystemStatus>> {
    let (flows, protocols, ports) = match state.storage.get_latest_snapshot() {
        Ok(data) => data,
        Err(_) => (Vec::new(), Vec::new(), Vec::new()),
    };

    let status = SystemStatus {
        active_flows: flows.len(),
        monitored_ips: protocols.len(),
        active_ports: ports.len(),
        last_updated: Utc::now(),
        storage_status: "正常".to_string(),
    };

    Json(ApiResponse::success(status))
}

// 系统状态结构
#[derive(Serialize)]
pub struct SystemStatus {
    pub active_flows: usize,
    pub monitored_ips: usize,
    pub active_ports: usize,
    pub last_updated: DateTime<Utc>,
    pub storage_status: String,
}

// 创建 API 路由器
pub fn create_router(state: AppState) -> Router {
    Router::new()
        // 实时数据接口
        .route("/api/dashboard", get(get_dashboard))
        .route("/api/status", get(get_system_status))
        // 历史数据查询接口
        .route("/api/ip/history", get(get_ip_history))
        .route("/api/ip/protocols", get(get_ip_protocols))
        .route("/api/ports/top", get(get_top_ports))
        // 监听配置接口
        .route("/api/listeners", get(get_listeners))
        .route("/api/listeners/ip", post(add_listener_ip))
        .route("/api/listeners/ip/remove", post(remove_listener_ip))
        .route("/api/listeners/port", post(add_listener_port))
        .route("/api/listeners/port/remove", post(remove_listener_port))
        // 健康检查
        .route("/health", get(health_check))
        // 启用 CORS
        .layer(CorsLayer::permissive())
        .with_state(state)
}

/// 健康检查端点
pub async fn health_check() -> Json<ApiResponse<HashMap<String, String>>> {
    let mut health = HashMap::new();
    health.insert("status".to_string(), "healthy".to_string());
    health.insert("service".to_string(), "tc-network-monitor".to_string());

    Json(ApiResponse::success(health))
}

// === 监听配置 API 处理函数 ===

/// 获取当前监听配置
pub async fn get_listeners(
    State(state): State<AppState>,
) -> Result<Json<ApiResponse<ListenerConfigResponse>>, StatusCode> {
    let config = state.listener_config.get_config().await;
    Ok(Json(ApiResponse::success(config)))
}

/// 添加监听 IP 地址
pub async fn add_listener_ip(
    State(state): State<AppState>,
    Json(request): Json<AddListenerIpRequest>,
) -> Result<Json<ApiResponse<ListenerOperationResult>>, StatusCode> {
    // 输入验证
    if let Err(e) = validate_ip_address(&request.ip) {
        return Ok(Json(ApiResponse::error(e.to_string())));
    }

    match state.listener_config.add_listen_ip(&request.ip).await {
        Ok(result) => Ok(Json(ApiResponse::success(result))),
        Err(e) => {
            eprintln!("添加监听 IP 错误: {}", e);
            Ok(Json(ApiResponse::error(format!("添加监听 IP 失败: {}", e))))
        }
    }
}

/// 移除监听 IP 地址
pub async fn remove_listener_ip(
    State(state): State<AppState>,
    Json(request): Json<RemoveListenerIpRequest>,
) -> Result<Json<ApiResponse<ListenerOperationResult>>, StatusCode> {
    // 输入验证
    if let Err(e) = validate_ip_address(&request.ip) {
        return Ok(Json(ApiResponse::error(e.to_string())));
    }

    match state.listener_config.remove_listen_ip(&request.ip).await {
        Ok(result) => Ok(Json(ApiResponse::success(result))),
        Err(e) => {
            eprintln!("移除监听 IP 错误: {}", e);
            Ok(Json(ApiResponse::error(format!("移除监听 IP 失败: {}", e))))
        }
    }
}

/// 添加监听端口
pub async fn add_listener_port(
    State(state): State<AppState>,
    Json(request): Json<AddListenerPortRequest>,
) -> Result<Json<ApiResponse<ListenerOperationResult>>, StatusCode> {
    // 输入验证
    if let Err(e) = validate_port(request.port) {
        return Ok(Json(ApiResponse::error(e.to_string())));
    }

    match state.listener_config.add_listen_port(request.port).await {
        Ok(result) => Ok(Json(ApiResponse::success(result))),
        Err(e) => {
            eprintln!("添加监听端口错误: {}", e);
            Ok(Json(ApiResponse::error(format!("添加监听端口失败: {}", e))))
        }
    }
}

/// 移除监听端口
pub async fn remove_listener_port(
    State(state): State<AppState>,
    Json(request): Json<RemoveListenerPortRequest>,
) -> Result<Json<ApiResponse<ListenerOperationResult>>, StatusCode> {
    // 输入验证
    if let Err(e) = validate_port(request.port) {
        return Ok(Json(ApiResponse::error(e.to_string())));
    }

    match state.listener_config.remove_listen_port(request.port).await {
        Ok(result) => Ok(Json(ApiResponse::success(result))),
        Err(e) => {
            eprintln!("移除监听端口错误: {}", e);
            Ok(Json(ApiResponse::error(format!("移除监听端口失败: {}", e))))
        }
    }
}

// 辅助函数

/// 解析时间范围参数
fn parse_time_range(query: &TimeRangeQuery) -> (DateTime<Utc>, DateTime<Utc>) {
    let end_time = query.end.unwrap_or_else(Utc::now);
    let start_time = query.start.unwrap_or_else(|| {
        let hours = query.hours.unwrap_or(1);
        end_time - Duration::hours(hours)
    });

    (start_time, end_time)
}

/// 简单的 IP 地址验证
fn is_valid_ip(ip: &str) -> bool {
    ip.parse::<std::net::Ipv4Addr>().is_ok()
}

/// 将 IP 字符串转换为 u32
fn ip_str_to_u32(ip: &str) -> Result<u32, Box<dyn std::error::Error>> {
    let addr: std::net::Ipv4Addr = ip.parse()?;
    Ok(u32::from(addr))
}

// API 使用示例
pub async fn start_web_server(
    state: AppState,
    port: u16,
) -> Result<(), Box<dyn std::error::Error>> {
    let app = create_router(state);

    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port)).await?;
    println!("🚀 Web API 服务器启动在端口 {}", port);
    println!(
        "📊 访问 http://localhost:{}/api/dashboard 查看实时数据",
        port
    );
    println!("⚙️  访问 http://localhost:{}/api/listeners 查看监听配置", port);
    println!("❤️  访问 http://localhost:{}/health 进行健康检查", port);

    axum::serve(listener, app).await?;

    Ok(())
}
