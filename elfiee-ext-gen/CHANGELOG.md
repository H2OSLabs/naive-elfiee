# Changelog

All notable changes to elfiee-ext-gen will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0] - 2025-11-02

### Added

- 初始版本发布
- **核心功能**:
  - 扩展骨架生成 (`create` 命令)
  - 智能字段推断（基于能力名称自动推断 Payload 字段）
  - TDD 测试生成（Payload、功能、授权、工作流测试）
  - 开发指南生成 (`guide` 命令)
  - 扩展验证 (`validate` 命令)
- **测试生成**:
  - Payload 反序列化测试
  - 基本功能测试
  - CBAC 授权测试（所有者/非所有者/授权场景）
  - 工作流集成测试
- **自动注册**:
  - 自动更新 `src/extensions/mod.rs`
  - 自动更新 `src/capabilities/registry.rs`
  - 自动更新 `src/lib.rs` (specta types)
- **模板系统**:
  - `mod.rs.tera` - 模块定义和 Payload 结构
  - `capability.rs.tera` - 能力处理器骨架
  - `tests.rs.tera` - 完整测试套件
  - `DEVELOPMENT_GUIDE.md.tera` - 开发指南
- **命令行参数**:
  - `-n, --name`: 扩展名称
  - `-b, --block-type`: 块类型
  - `-c, --capabilities`: 能力列表（逗号分隔）
  - `--with-auth-tests`: 生成授权测试（默认 true）
  - `--with-workflow-tests`: 生成工作流测试（默认 true）

### Fixed

- 模板导出模式修正：使用通配符 `pub use *` 而非显式结构体导出
- 测试模块导入路径修正：`CapabilityRegistry` 路径修正为 `registry::CapabilityRegistry`

### Documentation

- 完整的 README.md 包含安装、使用、TDD 工作流说明
- 生成器开发计划文档 (generator-dev-plan.md)
- 生成器设计文档 (generator-work-design.md)
- 开发进度跟踪文档 (progress.md)

### Known Limitations

- 需要从源码安装 (`cargo install --path .`)
- 未来版本将发布到 crates.io

## [Unreleased]

### Planned

- 发布到 crates.io
- 支持全局安装 `cargo install elfiee-ext-gen`

---

[0.1.0]: https://github.com/yourorg/elfiee/releases/tag/elfiee-ext-gen-v0.1.0
[Unreleased]: https://github.com/yourorg/elfiee/compare/elfiee-ext-gen-v0.1.0...HEAD
