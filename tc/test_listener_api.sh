#!/bin/bash

# 监听配置 API 测试脚本
# 使用 curl 测试新添加的监听配置 API 端点

BASE_URL="http://localhost:8080"

echo "=== 监听配置 API 测试脚本 ==="
echo "基础 URL: $BASE_URL"
echo ""

# 检查服务器是否运行
echo "1. 检查服务器状态..."
curl -s "$BASE_URL/health" | jq . || {
    echo "❌ 服务器未运行或无法访问"
    echo "请先启动 auxm 服务器: sudo ./target/release/tc --iface eth0"
    exit 1
}
echo "✅ 服务器运行正常"
echo ""

# 获取当前监听配置
echo "2. 获取当前监听配置..."
curl -s "$BASE_URL/api/listeners" | jq .
echo ""

# 添加监听 IP 地址
echo "3. 添加监听 IP 地址 (192.168.1.200)..."
curl -s -X POST "$BASE_URL/api/listeners/ip" \
  -H "Content-Type: application/json" \
  -d '{"ip": "192.168.1.200"}' | jq .
echo ""

# 添加监听端口
echo "4. 添加监听端口 (9090)..."
curl -s -X POST "$BASE_URL/api/listeners/port" \
  -H "Content-Type: application/json" \
  -d '{"port": 9090}' | jq .
echo ""

# 再次获取监听配置，验证更改
echo "5. 验证配置更改..."
curl -s "$BASE_URL/api/listeners" | jq .
echo ""

# 测试重复添加（应该失败）
echo "6. 测试重复添加 IP (应该失败)..."
curl -s -X POST "$BASE_URL/api/listeners/ip" \
  -H "Content-Type: application/json" \
  -d '{"ip": "192.168.1.200"}' | jq .
echo ""

# 测试无效 IP 地址
echo "7. 测试无效 IP 地址 (应该失败)..."
curl -s -X POST "$BASE_URL/api/listeners/ip" \
  -H "Content-Type: application/json" \
  -d '{"ip": "invalid.ip.address"}' | jq .
echo ""

# 测试无效端口
echo "8. 测试无效端口 (应该失败)..."
curl -s -X POST "$BASE_URL/api/listeners/port" \
  -H "Content-Type: application/json" \
  -d '{"port": 0}' | jq .
echo ""

# 移除监听 IP
echo "9. 移除监听 IP (192.168.1.200)..."
curl -s -X POST "$BASE_URL/api/listeners/ip/remove" \
  -H "Content-Type: application/json" \
  -d '{"ip": "192.168.1.200"}' | jq .
echo ""

# 移除监听端口
echo "10. 移除监听端口 (9090)..."
curl -s -X POST "$BASE_URL/api/listeners/port/remove" \
  -H "Content-Type: application/json" \
  -d '{"port": 9090}' | jq .
echo ""

# 最终验证
echo "11. 最终配置验证..."
curl -s "$BASE_URL/api/listeners" | jq .
echo ""

# 测试移除不存在的项目
echo "12. 测试移除不存在的 IP (应该失败)..."
curl -s -X POST "$BASE_URL/api/listeners/ip/remove" \
  -H "Content-Type: application/json" \
  -d '{"ip": "10.10.10.10"}' | jq .
echo ""

echo "=== 测试完成 ==="
echo ""
echo "📝 API 端点总结:"
echo "  GET  /api/listeners              - 获取当前监听配置"
echo "  POST /api/listeners/ip           - 添加监听 IP"
echo "  POST /api/listeners/ip/remove    - 移除监听 IP"
echo "  POST /api/listeners/port         - 添加监听端口"
echo "  POST /api/listeners/port/remove  - 移除监听端口"
echo ""
echo "🔧 使用示例:"
echo "  # 添加 IP"
echo "  curl -X POST http://localhost:8080/api/listeners/ip \\"
echo "    -H 'Content-Type: application/json' \\"
echo "    -d '{\"ip\": \"192.168.1.100\"}'"
echo ""
echo "  # 添加端口"
echo "  curl -X POST http://localhost:8080/api/listeners/port \\"
echo "    -H 'Content-Type: application/json' \\"
echo "    -d '{\"port\": 8080}'"
