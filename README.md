# tc - eBPF-based Network Traffic Monitoring Tool

[中文版本](README_zh.md) | English

A high-performance network traffic monitoring tool built with Rust and eBPF technology, capable of real-time monitoring of network traffic for specified IP addresses at the kernel level.

## 🚀 Features

- **High-Performance Monitoring**: Uses XDP (eXpress Data Path) technology to process network packets at the kernel level
- **Real-time Traffic Statistics**: Monitors inbound and outbound traffic (packet count and bytes) for specified IP addresses
- **Flexible Configuration**: Configure monitored IP addresses via environment variables
- **Detailed Logging**: Provides comprehensive traffic statistics and packet processing logs
- **Cross-Platform Support**: Supports Linux environments, runs on x86_64 and ARM64 architectures

## 📋 System Requirements

- **Operating System**: Linux (kernel version 5.8+, with XDP support)
- **Permissions**: Requires root privileges to load eBPF programs
- **Network Interface**: Network interface that supports XDP

## 🛠️ Prerequisites

### Development Environment

1. **Rust Toolchains**: 
   - Stable version: `rustup toolchain install stable`
   - Nightly version: `rustup toolchain install nightly --component rust-src`

2. **Cross-compilation Support** (if needed):
   - Add target architecture: `rustup target add ${ARCH}-unknown-linux-musl`
   - LLVM toolchain: `brew install llvm` (macOS)
   - C toolchain: [`brew install filosottile/musl-cross/musl-cross`](https://github.com/FiloSottile/homebrew-musl-cross) (macOS)

3. **eBPF Linker**:
   ```shell
   cargo install bpf-linker
   ```
   > Note: Use `cargo install bpf-linker --no-default-features` on macOS

## 🏗️ Project Architecture

```
tc/
├── tc/           # Main program (userspace)
│   ├── src/
│   │   ├── main.rs      # Program entry point
│   │   ├── target_ip.rs # IP address handling
│   │   └── utils.rs     # Utility functions
│   └── Cargo.toml
├── tc-ebpf/      # eBPF program (kernel space)
│   ├── src/
│   │   ├── main.rs      # XDP program implementation
│   │   └── lib.rs       # Library file
│   └── Cargo.toml
├── tc-common/    # Shared data structures
│   ├── src/
│   │   ├── lib.rs       # Common data structures
│   │   └── utils.rs     # Utility functions
│   └── Cargo.toml
└── Cargo.toml    # Workspace configuration
```

## ⚙️ Configuration

### Environment Variables

Create a `.env` file or set environment variables:

```bash
# Target IP addresses to monitor, separated by commas
TARGET_IP=192.168.1.100,10.0.0.1,172.16.1.50
```

### Network Interface

By default, monitors the `eth0` interface. You can modify it via command line arguments:

```shell
# Specify network interface
sudo ./target/release/tc --iface ens18
```

## 🚀 Build & Run

### Development Build

```shell
# Check code
cargo check

# Build project
cargo build --release

# Run program (requires root privileges)
cargo run --release --config 'target."cfg(all())".runner="sudo -E"'
```

### Manual Execution

```shell
# Set environment variables
export TARGET_IP=192.168.1.100,10.0.0.1

# Run program
sudo -E ./target/release/tc --iface eth0
```

### Cross-compilation (on macOS)

Cross compilation works on both Intel and Apple Silicon Macs.

```shell
# Set target architecture
export ARCH=x86_64  # or aarch64

# Cross compile
CC=${ARCH}-linux-musl-gcc cargo build --package tc --release \
  --target=${ARCH}-unknown-linux-musl \
  --config=target.${ARCH}-unknown-linux-musl.linker=\"${ARCH}-linux-musl-gcc\"
```

The cross-compiled program `target/${ARCH}-unknown-linux-musl/release/tc` can be copied to a Linux server or VM and run there.

## 📊 Usage Examples

### Basic Usage

```shell
# 1. Set IP addresses to monitor
export TARGET_IP=192.168.1.100

# 2. Run monitoring program
sudo -E ./target/release/tc --iface eth0
```

### Output Example

```
[INFO] XDP program loaded and attached to eth0 interface
[INFO] Starting traffic monitoring for [192.168.1.100]...
[INFO] Press Ctrl-C to exit

=== Traffic Statistics for 192.168.1.100 ===
Inbound Traffic:
  Packets: 1247
  Bytes: 89432 bytes (87.34 KB)
Outbound Traffic:
  Packets: 1156
  Bytes: 156789 bytes (153.11 KB)
Total:
  Packets: 2403
  Bytes: 246221 bytes (240.45 KB)
================================
```

### Monitor Multiple IPs

```shell
# Monitor multiple IP addresses
export TARGET_IP=192.168.1.100,10.0.0.1,172.16.1.50
sudo -E ./target/release/tc --iface eth0
```

## 🔧 Technical Details

### eBPF Program

- **Program Type**: XDP (eXpress Data Path)
- **Packet Processing**: Processes packets at the network driver level
- **Performance Advantage**: Avoids the overhead of the kernel network stack

### Data Structures

```rust
// Traffic statistics structure
pub struct TrafficStats {
    pub inbound_packets: u64,   // Number of inbound packets
    pub inbound_bytes: u64,     // Inbound bytes
    pub outbound_packets: u64,  // Number of outbound packets
    pub outbound_bytes: u64,    // Outbound bytes
}
```

### eBPF Maps

- `TARGET_IP`: Stores IP addresses to monitor
- `TRAFFIC_STATS`: Stores traffic statistics for each IP

## 🐛 Troubleshooting

### Common Issues

1. **Permission Error**: Ensure running with root privileges
2. **Network Interface Not Supported**: Confirm the network interface supports XDP
3. **Kernel Version**: Ensure Linux kernel version >= 5.8

### Debug Mode

```shell
# Enable verbose logging
RUST_LOG=debug sudo -E ./target/release/tc --iface eth0
```

### XDP Mode Switching

If the default XDP mode doesn't work, try modifying the XDP flags in the code:

```rust
// Change XdpFlags::default() to XdpFlags::SKB_MODE
program.attach(&opt.iface, XdpFlags::SKB_MODE)
```

## 🤝 Contributing

Contributions are welcome! Please follow these steps:

1. Fork the project
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Create a Pull Request

## 📄 License

With the exception of eBPF code, tc is distributed under the terms of either the [MIT license](LICENSE-MIT) or the [Apache License](LICENSE-APACHE) (version 2.0), at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this crate by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

### eBPF

All eBPF code is distributed under either the terms of the [GNU General Public License, Version 2](LICENSE-GPL2) or the [MIT license](LICENSE-MIT), at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this project by you, as defined in the GPL-2 license, shall be dual licensed as above, without any additional terms or conditions.

---

## 🙏 Acknowledgments

This project is built on the [Aya](https://github.com/aya-rs/aya) framework. Thanks to the Aya team for providing excellent eBPF development tools. 