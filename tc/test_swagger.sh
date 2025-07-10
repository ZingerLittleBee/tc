#!/bin/bash

# TC Network Monitor Swagger 文档测试脚本
# 
# 此脚本用于测试 Swagger API 文档的生成和访问

set -e

echo "🚀 开始测试 TC Network Monitor Swagger 文档..."

# 检查项目是否能编译
echo "📦 检查项目编译..."
cargo check --quiet
if [ $? -eq 0 ]; then
    echo "✅ 项目编译成功"
else
    echo "❌ 项目编译失败"
    exit 1
fi

# 构建项目
echo "🔨 构建项目..."
cargo build --quiet
if [ $? -eq 0 ]; then
    echo "✅ 项目构建成功"
else
    echo "❌ 项目构建失败"
    exit 1
fi

# 启动服务器（后台运行）
echo "🌐 启动 Web API 服务器..."
cargo run --bin tc -- --web-only &
SERVER_PID=$!

# 等待服务器启动
echo "⏳ 等待服务器启动..."
sleep 5

# 检查服务器是否启动成功
if ! kill -0 $SERVER_PID 2>/dev/null; then
    echo "❌ 服务器启动失败"
    exit 1
fi

echo "✅ 服务器启动成功 (PID: $SERVER_PID)"

# 测试健康检查端点
echo "🔍 测试健康检查端点..."
if curl -s http://localhost:8080/health > /dev/null; then
    echo "✅ 健康检查端点正常"
else
    echo "❌ 健康检查端点失败"
    kill $SERVER_PID
    exit 1
fi

# 测试 OpenAPI JSON 端点
echo "📋 测试 OpenAPI JSON 端点..."
if curl -s http://localhost:8080/api-docs/openapi.json | jq . > /dev/null 2>&1; then
    echo "✅ OpenAPI JSON 端点正常"
else
    echo "❌ OpenAPI JSON 端点失败"
    kill $SERVER_PID
    exit 1
fi

# 测试 Swagger UI 端点
echo "📚 测试 Swagger UI 端点..."
if curl -s http://localhost:8080/swagger-ui/ | grep -q "swagger-ui"; then
    echo "✅ Swagger UI 端点正常"
else
    echo "❌ Swagger UI 端点失败"
    kill $SERVER_PID
    exit 1
fi

# 获取 OpenAPI 规范并验证内容
echo "🔍 验证 OpenAPI 规范内容..."
OPENAPI_JSON=$(curl -s http://localhost:8080/api-docs/openapi.json)

# 检查基本信息
if echo "$OPENAPI_JSON" | jq -e '.info.title' | grep -q "TC Network Monitor API"; then
    echo "✅ API 标题正确"
else
    echo "❌ API 标题不正确"
fi

if echo "$OPENAPI_JSON" | jq -e '.info.version' | grep -q "0.1.0"; then
    echo "✅ API 版本正确"
else
    echo "❌ API 版本不正确"
fi

# 检查路径
PATHS=$(echo "$OPENAPI_JSON" | jq -r '.paths | keys[]')
echo "📍 发现的 API 路径:"
echo "$PATHS" | while read path; do
    echo "  - $path"
done

# 检查关键端点是否存在
if echo "$OPENAPI_JSON" | jq -e '.paths."/api/dashboard"' > /dev/null; then
    echo "✅ 仪表板端点已文档化"
else
    echo "❌ 仪表板端点未文档化"
fi

if echo "$OPENAPI_JSON" | jq -e '.paths."/api/status"' > /dev/null; then
    echo "✅ 状态端点已文档化"
else
    echo "❌ 状态端点未文档化"
fi

if echo "$OPENAPI_JSON" | jq -e '.paths."/health"' > /dev/null; then
    echo "✅ 健康检查端点已文档化"
else
    echo "❌ 健康检查端点未文档化"
fi

# 检查组件/模式
if echo "$OPENAPI_JSON" | jq -e '.components.schemas' > /dev/null; then
    echo "✅ 数据模式已定义"
    SCHEMAS=$(echo "$OPENAPI_JSON" | jq -r '.components.schemas | keys[]')
    echo "📊 发现的数据模式:"
    echo "$SCHEMAS" | while read schema; do
        echo "  - $schema"
    done
else
    echo "❌ 数据模式未定义"
fi

# 停止服务器
echo "🛑 停止服务器..."
kill $SERVER_PID
wait $SERVER_PID 2>/dev/null || true

echo ""
echo "🎉 Swagger 文档测试完成！"
echo ""
echo "📚 访问方式："
echo "  - Swagger UI: http://localhost:8080/swagger-ui"
echo "  - OpenAPI JSON: http://localhost:8080/api-docs/openapi.json"
echo "  - 健康检查: http://localhost:8080/health"
echo ""
echo "🚀 要启动服务器，请运行："
echo "  cargo run --bin tc -- --web-only"
