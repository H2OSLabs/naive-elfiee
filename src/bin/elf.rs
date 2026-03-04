//! elf — Elfiee CLI (EventWeaver for .elf projects)
//!
//! 统一 CLI 入口，提供 init, register, unregister, serve, run, status, scan, block, event, grant, revoke 子命令。
//!
//! Usage:
//!   elf init [project]
//!   elf register <agent_type> [--name <name>] [--config-dir <dir>]
//!   elf serve [--port 47200] [--project <path>]
//!   elf run <template> [--project <path>] [--port 47200]
//!   elf status [project]
//!   elf scan [--project <path>]
//!   elf block list [--project <path>]
//!   elf event list [--project <path>]
//!   elf event history <block> [--project <path>]
//!   elf event at <block> <event_id> [--project <path>]
//!   elf grant <editor_id> <capability> [block] [--project <path>]
//!   elf revoke <editor_id> <capability> [block] [--project <path>]

use clap::{Parser, Subcommand};
use elfiee_lib::mcp;
use elfiee_lib::services;
use elfiee_lib::state::AppState;
use std::sync::Arc;

#[derive(Parser)]
#[command(
    name = "elf",
    about = "Elfiee CLI — EventWeaver for .elf projects",
    version = env!("CARGO_PKG_VERSION")
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 初始化 .elf/ 项目目录
    Init {
        /// 项目路径（默认当前目录）
        #[arg(default_value = ".")]
        project: String,
    },

    /// 注册 Agent（创建 Editor + Grants + 注入 MCP 配置 + Skill）
    Register {
        /// Agent 类型：claude, openclaw, custom
        #[arg(default_value = "claude")]
        agent_type: String,

        /// Agent 名称
        #[arg(long)]
        name: Option<String>,

        /// Agent 配置目录（默认推断）
        #[arg(long)]
        config_dir: Option<String>,

        /// 项目路径
        #[arg(long, default_value = ".")]
        project: String,

        /// MCP 端口
        #[arg(long, default_value = "47200")]
        port: u16,
    },

    /// 取消注册 Agent（删除 Editor + 清理注入的配置）
    Unregister {
        /// 要取消注册的 Editor ID
        editor_id: String,

        /// Agent 配置目录（默认 .claude）
        #[arg(long)]
        config_dir: Option<String>,

        /// 项目路径
        #[arg(long, default_value = ".")]
        project: String,
    },

    /// 启动 MCP SSE Server
    Serve {
        /// MCP SSE server 端口
        #[arg(long, default_value = "47200")]
        port: u16,

        /// 启动时预加载的项目路径
        #[arg(long)]
        project: Option<String>,
    },

    /// 按 Socialware 模板注册角色并启动 MCP server
    Run {
        /// 模板名（在 .elf/templates/workflows/ 或内置模板中查找）
        template: String,

        /// 项目路径
        #[arg(long, default_value = ".")]
        project: String,

        /// MCP 端口
        #[arg(long, default_value = "47200")]
        port: u16,
    },

    /// 查看项目状态
    Status {
        /// 项目路径（默认当前目录）
        #[arg(default_value = ".")]
        project: String,
    },

    /// 扫描并同步文件内容到 Elfiee block（省略文件则扫描全部）
    Scan {
        /// 指定单个文件路径（省略则批量扫描全部）
        file: Option<String>,

        /// 项目路径
        #[arg(long, default_value = ".")]
        project: String,
    },

    /// Block 管理子命令
    Block {
        #[command(subcommand)]
        action: BlockCommands,
    },

    /// Event 查询子命令（list / history / at）
    Event {
        #[command(subcommand)]
        action: EventCommands,
    },

    /// 授予 Editor 对 Block 的 Capability 权限
    Grant {
        /// Editor ID
        editor_id: String,

        /// Capability ID（如 document.write, task.read）
        capability: String,

        /// Block（name 或 id，默认 "*" 表示所有 blocks）
        #[arg(default_value = "*")]
        block: String,

        /// 项目路径
        #[arg(long, default_value = ".")]
        project: String,
    },

    /// 撤回 Editor 对 Block 的 Capability 权限
    Revoke {
        /// Editor ID
        editor_id: String,

        /// Capability ID
        capability: String,

        /// Block（name 或 id，默认 "*" 表示所有 blocks）
        #[arg(default_value = "*")]
        block: String,

        /// 项目路径
        #[arg(long, default_value = ".")]
        project: String,
    },
}

#[derive(Subcommand)]
enum BlockCommands {
    /// 列出所有 blocks
    List {
        /// 项目路径
        #[arg(long, default_value = ".")]
        project: String,
    },

    /// 查看单个 block 的详细信息（ID、内容、关系）
    Get {
        /// Block name 或 id
        block: String,

        /// 项目路径
        #[arg(long, default_value = ".")]
        project: String,
    },
}

#[derive(Subcommand)]
enum EventCommands {
    /// 列出所有事件（CBAC 过滤）
    List {
        /// 项目路径
        #[arg(long, default_value = ".")]
        project: String,
    },

    /// 查看指定 block 的事件历史
    History {
        /// Block name 或 id
        block: String,

        /// 项目路径
        #[arg(long, default_value = ".")]
        project: String,
    },

    /// 时间旅行：查看 block 在指定 event 时刻的状态
    At {
        /// Block name 或 id
        block: String,

        /// Event ID（目标时间点）
        event_id: String,

        /// 项目路径
        #[arg(long, default_value = ".")]
        project: String,
    },
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Init { project } => elfiee_lib::cli::init::run(&project).await,

        Commands::Register {
            agent_type,
            name,
            config_dir,
            project,
            port,
        } => elfiee_lib::cli::register::run(
            &agent_type,
            name.as_deref(),
            config_dir.as_deref(),
            &project,
            port,
        )
        .await
        .map(|_| ()),

        Commands::Unregister {
            editor_id,
            config_dir,
            project,
        } => elfiee_lib::cli::unregister::run(&editor_id, config_dir.as_deref(), &project).await,

        Commands::Serve { port, project } => run_serve(port, project.as_deref()).await,

        Commands::Run {
            template,
            project,
            port,
        } => elfiee_lib::cli::run::run(&template, &project, port).await,

        Commands::Status { project } => elfiee_lib::cli::status::run(&project).await,

        Commands::Scan { file, project } => {
            elfiee_lib::cli::scan::run(&project, file.as_deref()).await
        }

        Commands::Block { action } => match action {
            BlockCommands::List { project } => elfiee_lib::cli::block::list(&project).await,
            BlockCommands::Get { block, project } => {
                elfiee_lib::cli::block::get(&project, &block).await
            }
        },

        Commands::Event { action } => match action {
            EventCommands::List { project } => elfiee_lib::cli::event::list(&project).await,
            EventCommands::History { block, project } => {
                elfiee_lib::cli::event::history(&project, &block).await
            }
            EventCommands::At {
                block,
                event_id,
                project,
            } => elfiee_lib::cli::event::at(&project, &block, &event_id).await,
        },

        Commands::Grant {
            editor_id,
            capability,
            block,
            project,
        } => elfiee_lib::cli::grant::run(&project, &editor_id, &capability, &block).await,

        Commands::Revoke {
            editor_id,
            capability,
            block,
            project,
        } => elfiee_lib::cli::revoke::run(&project, &editor_id, &capability, &block).await,
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}

/// `elf serve` — 启动 MCP SSE Server
async fn run_serve(port: u16, project: Option<&str>) -> Result<(), String> {
    let app_state = Arc::new(AppState::new());

    println!("elf v{}", env!("CARGO_PKG_VERSION"));
    println!("Elfiee MCP server starting...");

    // 预加载项目
    if let Some(project_path) = project {
        match services::project::open_project(project_path, &app_state).await {
            Ok(file_id) => {
                println!("Opened project: {} (file_id: {})", project_path, file_id);
            }
            Err(e) => {
                return Err(format!("Failed to open project '{}': {}", project_path, e));
            }
        }
    }

    // 启动 MCP SSE server
    mcp::start_mcp_server(app_state, port)
        .await
        .map_err(|e| format!("Failed to start MCP server: {}", e))?;

    println!();
    println!(
        "Ready. Clients can connect via MCP SSE at http://127.0.0.1:{}",
        port
    );
    println!("  1. Call elfiee_auth to authenticate");
    println!("  2. Call elfiee_open to open a project");
    println!("  3. Use block/document/task tools to operate");
    println!();
    println!("Press Ctrl+C to stop.");

    tokio::signal::ctrl_c()
        .await
        .map_err(|e| format!("Failed to listen for ctrl-c: {}", e))?;

    println!("\nShutting down...");
    Ok(())
}
