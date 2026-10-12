title:: OpenSeq 架构
tags:: openseq, architecture

- 三层：TypeScript UI → Tauri 命令层 → `logseq-core`（纯 Rust）
  - `crates/core` 完全不依赖 Tauri，所以能用 `cargo test` 单独验证。
  - 前端只负责「编辑」（光标、按键驱动的树操作）。
  - Rust 只负责「意义」（解析、序列化、索引、写盘）。
- 一个不变式：**Markdown 文件是唯一事实来源**，内存里的一切都只是缓存，随时可以丢掉重建。
- 写盘用「临时文件 + rename」，断电也不会写坏页面。
- 返回 [[Features]] 或 [[Welcome to OpenSeq]]。
