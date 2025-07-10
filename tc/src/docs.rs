//! Swagger/OpenAPI 文档定义

use utoipa::OpenApi;

/// 主要的 OpenAPI 规范定义
#[derive(OpenApi)]
#[openapi(
    info(
        title = "TC Network Monitor API",
        version = "0.1.0"
    ),
    paths(
        crate::web_api::get_dashboard,
        crate::web_api::get_system_status,
        crate::web_api::health_check,
    ),
    tags(
        (name = "dashboard"),
        (name = "system")
    )
)]
pub struct ApiDoc;
