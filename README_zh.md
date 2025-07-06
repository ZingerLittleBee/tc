# tc - 基于 eBPF 的网络流量监控工具

中文版本 | [English](README.md)

一个使用 Rust 和 eBPF 技术构建的高性能网络流量监控工具，能够在内核层面实时监控指定 IP 地址的网络流量。

## 🚀 功能特性

- **高性能监控**: 使用 XDP (eXpress Data Path) 技术在内核层面处理网络数据包
- **实时流量统计**: 监控指定 IP 地址的入站和出站流量（数据包数量和字节数）
- **灵活配置**: 通过环境变量配置要监控的 IP 地址
- **详细日志**: 提供详细的流量统计信息和数据包处理日志
- **跨平台支持**: 支持 Linux 环境，可在 x86_64 和 ARM64 架构上运行

## 📋 系统要求

- **操作系统**: Linux (内核版本 5.8+，支持 XDP)
- **权限**: 需要 root 权限来加载 eBPF 程序
- **网络接口**: 支持 XDP 的网络接口

## 🛠️ 依赖环境

### 开发环境

1. **Rust 工具链**: 
   - 稳定版本: `rustup toolchain install stable`
   - 夜间版本: `rustup toolchain install nightly --component rust-src`

2. **交叉编译支持** (如果需要):
   - 添加目标架构: `rustup target add ${ARCH}-unknown-linux-musl`
   - LLVM 工具链: `brew install llvm` (macOS)
   - C 工具链: [`brew install filosottile/musl-cross/musl-cross`](https://github.com/FiloSottile/homebrew-musl-cross) (macOS)

3. **eBPF 链接器**:
   ```shell
   cargo install bpf-linker
   ```
   > 注：在 macOS 上请使用 `cargo install bpf-linker --no-default-features`

## 🏗️ 项目架构

```
tc/
├── tc/           # 主程序 (用户空间)
│   ├── src/
│   │   ├── main.rs      # 程序入口点
│   │   ├── target_ip.rs # IP 地址处理
│   │   └── utils.rs     # 工具函数
│   └── Cargo.toml
├── tc-ebpf/      # eBPF 程序 (内核空间)
│   ├── src/
│   │   ├── main.rs      # XDP 程序实现
│   │   └── lib.rs       # 库文件
│   └── Cargo.toml
├── tc-common/    # 共享数据结构
│   ├── src/
│   │   ├── lib.rs       # 通用数据结构
│   │   └── utils.rs     # 工具函数
│   └── Cargo.toml
└── Cargo.toml    # 工作空间配置
```

## ⚙️ 配置说明

### 环境变量

创建 `.env` 文件或设置环境变量：

```bash
# 监控的目标 IP 地址，多个 IP 用逗号分隔
TARGET_IP=192.168.1.100,10.0.0.1,172.16.1.50
```

### 网络接口

默认监控 `eth0` 接口，可以通过命令行参数修改：

```shell
# 指定网络接口
sudo ./target/release/tc --iface ens18
```

## 🚀 构建和运行

### 开发环境构建

```shell
# 检查代码
cargo check

# 构建项目
cargo build --release

# 运行程序 (需要 root 权限)
cargo run --release --config 'target."cfg(all())".runner="sudo -E"'
```

### 手动运行

```shell
# 设置环境变量
export TARGET_IP=192.168.1.100,10.0.0.1

# 运行程序
sudo -E ./target/release/tc --iface eth0
```

### 交叉编译 (在 macOS 上)

```shell
# 设置目标架构
export ARCH=x86_64  # 或 aarch64

# 交叉编译
CC=${ARCH}-linux-musl-gcc cargo build --package tc --release \
  --target=${ARCH}-unknown-linux-musl \
  --config=target.${ARCH}-unknown-linux-musl.linker=\"${ARCH}-linux-musl-gcc\"
```

编译后的程序位于 `target/${ARCH}-unknown-linux-musl/release/tc`，可以复制到 Linux 服务器运行。

## 📊 使用示例

### 基本使用

```shell
# 1. 设置要监控的 IP 地址
export TARGET_IP=192.168.1.100

# 2. 运行监控程序
sudo -E ./target/release/tc --iface eth0
```

### 输出示例

```
[INFO] XDP程序已加载并附加到 eth0 接口
[INFO] 开始监控 [192.168.1.100] 的流量...
[INFO] 按 Ctrl-C 退出

=== 流量统计 for 192.168.1.100 ===
入站流量:
  数据包: 1247 个
  字节数: 89432 bytes (87.34 KB)
出站流量:
  数据包: 1156 个
  字节数: 156789 bytes (153.11 KB)
总计:
  数据包: 2403 个
  字节数: 246221 bytes (240.45 KB)
================================
```

### 监控多个 IP

```shell
# 监控多个 IP 地址
export TARGET_IP=192.168.1.100,10.0.0.1,172.16.1.50
sudo -E ./target/release/tc --iface eth0
```

## 🔧 技术细节

### eBPF 程序

- **程序类型**: XDP (eXpress Data Path)
- **数据包处理**: 在网络驱动层面处理数据包
- **性能优势**: 避免了内核网络栈的开销

### 数据结构

```rust
// 流量统计结构
pub struct TrafficStats {
    pub inbound_packets: u64,   // 入站数据包数量
    pub inbound_bytes: u64,     // 入站字节数
    pub outbound_packets: u64,  // 出站数据包数量
    pub outbound_bytes: u64,    // 出站字节数
}
```

### 映射表

- `TARGET_IP`: 存储要监控的 IP 地址
- `TRAFFIC_STATS`: 存储每个 IP 的流量统计信息

## 🐛 故障排除

### 常见问题

1. **权限错误**: 确保以 root 权限运行程序
2. **网络接口不支持**: 确认网络接口支持 XDP
3. **内核版本**: 确保 Linux 内核版本 >= 5.8

### 调试模式

```shell
# 启用详细日志
RUST_LOG=debug sudo -E ./target/release/tc --iface eth0
```

### XDP 模式切换

如果默认 XDP 模式不工作，可以尝试修改代码中的 XDP 标志：

```rust
// 将 XdpFlags::default() 改为 XdpFlags::SKB_MODE
program.attach(&opt.iface, XdpFlags::SKB_MODE)
```

## 🤝 贡献指南

欢迎贡献代码！请遵循以下步骤：

1. Fork 项目
2. 创建特性分支 (`git checkout -b feature/amazing-feature`)
3. 提交更改 (`git commit -m 'Add amazing feature'`)
4. 推送到分支 (`git push origin feature/amazing-feature`)
5. 创建 Pull Request

## 📄 许可证

除 eBPF 代码外，本项目采用 [MIT 许可证](LICENSE-MIT) 或 [Apache 许可证 2.0](LICENSE-APACHE) 双重许可。

### eBPF 代码

所有 eBPF 代码采用 [GNU 通用公共许可证 v2](LICENSE-GPL2) 或 [MIT 许可证](LICENSE-MIT) 双重许可。

除非您明确声明，否则根据 Apache-2.0 许可证定义，您有意提交给本项目的任何贡献都将按照上述双重许可证进行许可，无任何附加条款或条件。

---

## 🙏 致谢

本项目基于 [Aya](https://github.com/aya-rs/aya) 框架构建，感谢 Aya 团队提供的优秀 eBPF 开发工具。
