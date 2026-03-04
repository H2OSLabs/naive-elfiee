//! 服务层 — 统一数据总线
//!
//! 所有传输层（MCP Server / CLI）统一通过此层访问 Engine。
//! 服务层封装 CBAC 过滤和业务逻辑，传输层只做格式适配。

pub mod block;
pub mod document;
pub mod editor;
pub mod event;
pub mod grant;
pub mod project;
pub mod session;
pub mod task;
