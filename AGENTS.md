# Repository Guidelines

## Project and Module Responsibilities

This project is a Linux eBPF/XDP command-line traffic statistics tool built with Rust and Aya, not the Linux `tc` command.

- `tc/`: Userspace application responsible for configuration, eBPF loading and attachment, map reads, and statistics output.
- `tc-ebpf/`: Kernel-side XDP program responsible for packet parsing and statistics. Keep it `no_std` and check bounds before accessing packet data.
- `tc-common/`: Common data structures and IPv4 conversion. Keep it `no_std`; the `user` feature enables userspace Aya integration.
- `README.md` and `README_zh.md`: English and Chinese usage documentation. Update both when configuration, commands, or behavior change.

## Userspace and Kernel Contracts

- `TrafficStats` is currently defined in both `tc-common/src/lib.rs` and `tc-ebpf/src/main.rs`. Keep `#[repr(C)]`, field order, field types, and layout identical on both sides.
- eBPF map names and key/value types must match the userspace readers. The current maps are `TARGET_IP` and `TRAFFIC_STATS`, both keyed by IPv4 addresses represented as `u32`.
- When changing IP conversion or packet parsing, verify that network byte order agrees with userspace configuration values.
- Account for concurrent updates across CPUs when changing statistics logic. A regular HashMap read-modify-write sequence is not atomic.

## Build and Verification

Building requires Rust stable, nightly with `rust-src`, and `bpf-linker` on PATH; see the README for installation instructions. `tc/build.rs` builds eBPF through `aya-build`, and the resulting artifact is embedded in the userspace application, so checking the main application also requires the eBPF toolchain.

Run the relevant commands from the repository root:

```sh
# Check formatting without modifying files
cargo fmt --all -- --check

# Basic checks and tests for common-library changes
cargo check -p tc-common
cargo test -p tc-common

# Application and eBPF integration check and release build
cargo check
cargo build --release
```

- Documentation-only changes require checking source references, paths, commands, and Markdown, not a build.
- Verify observable behavior for functional changes, preferably by extending existing tests. Add a regression test that reproduces the issue when fixing a bug.
- Successful compilation does not establish kernel verifier acceptance or correct statistics. Report the environment, results, and unverified behavior for actual loading and traffic tests separately. State any missing toolchain requirements.

## Runtime Safety and Current Limitations

- Do not attach XDP to the default interface or an interface carrying management connections, or replace an existing XDP program, without explicit authorization. Prefer isolated network namespaces and veth pairs for runtime tests, and clean up resources you create.
- Only the XDP receive path is currently attached. Source-IP matches are labeled inbound and destination-IP matches outbound; these labels do not represent complete bidirectional host traffic.
- IPv4 packets other than TCP/UDP currently return `XDP_ABORTED`, potentially dropping traffic such as ICMP. Do not describe the current implementation as passive monitoring without side effects.
- `TARGET_IP` parsing currently uses `unwrap()` for empty or invalid IPv4 input and can panic. Use valid, comma-separated IPv4 addresses without spaces in configuration examples.
- These limitations describe the current implementation, not behavior that must be preserved. Update this file and related documentation when they are fixed.

## Commit Guidelines

- Do not add `Amp-Thread-ID` or `Co-authored-by` Git commit trailers.
- Do not commit `.env`, credentials, build artifacts, or temporary debugging files.
