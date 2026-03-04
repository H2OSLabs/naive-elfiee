#!/usr/bin/env bash
# scan_unused_code.sh — 扫描 Elfiee Rust 后端中的无用代码
#
# 检测项：
# 1. 已删除模块的残留引用（directory, terminal, mcp transport）
# 2. 未被使用的 pub 函数（Handler 之外）
# 3. dead_code 编译器警告
# 4. 已注册但不存在的 capability
# 5. 过时的 use 语句

set -euo pipefail

PROJ_ROOT="${1:-$(git rev-parse --show-toplevel)}"
SRC_DIR="$PROJ_ROOT/src-tauri/src"
RED='\033[0;31m'
YELLOW='\033[0;33m'
GREEN='\033[0;32m'
NC='\033[0m'

echo "=========================================="
echo "  Elfiee 无用代码扫描"
echo "  扫描目录: $SRC_DIR"
echo "=========================================="
echo ""

ISSUES=0

# --- 1. 已删除模块的残留引用 ---
echo "--- [1/5] 检查已删除模块的残留引用 ---"
DELETED_MODULES=("extensions::directory" "extensions::terminal" "portable_pty" "TerminalState" "TerminalSession" "DirectoryContents" "FsScanner" "elf_meta" "terminal_sessions" "directory\.create" "directory\.delete" "directory\.import" "directory\.export" "directory\.write" "directory\.rename" "terminal\.init" "terminal\.execute" "terminal\.save" "terminal\.close" "task\.commit" "git_hooks" "mcp_config" "settings_config")

for module in "${DELETED_MODULES[@]}"; do
    matches=$(grep -rn "$module" "$SRC_DIR" --include="*.rs" \
        | grep -v "target/" \
        | grep -v "// TODO" \
        | grep -v "// DELETED" \
        | grep -v "// REMOVED" \
        || true)
    if [ -n "$matches" ]; then
        echo -e "${RED}[FOUND]${NC} 引用已删除模块 '$module':"
        echo "$matches" | head -10
        echo ""
        ISSUES=$((ISSUES + 1))
    fi
done

# --- 2. 检查 Handler 中的 I/O 操作 ---
echo "--- [2/5] 检查 Handler 中的禁止 I/O 操作 ---"
IO_PATTERNS=("std::fs::" "std::process::" "std::net::" "tokio::fs::" "tokio::process::" "Command::new" "File::create" "File::open" "fs::write" "fs::read" "fs::remove" "fs::create_dir" "symlink" "portable_pty")

# 只扫描 extensions/ 目录下的非测试文件
for pattern in "${IO_PATTERNS[@]}"; do
    matches=$(grep -rn "$pattern" "$SRC_DIR/extensions/" --include="*.rs" \
        | grep -v "tests.rs" \
        | grep -v "#\[cfg(test)\]" \
        | grep -v "mod tests" \
        | grep -v "// OK:" \
        || true)
    if [ -n "$matches" ]; then
        echo -e "${RED}[VIOLATION]${NC} Extension Handler 中发现 I/O 操作 '$pattern':"
        echo "$matches" | head -5
        echo ""
        ISSUES=$((ISSUES + 1))
    fi
done

# --- 3. cargo check 的 dead_code 警告 ---
echo "--- [3/5] 检查编译器 dead_code 警告 ---"
cd "$PROJ_ROOT/src-tauri"
DEAD_CODE=$(cargo check 2>&1 | grep -c "dead_code" || true)
if [ "$DEAD_CODE" -gt 0 ]; then
    echo -e "${YELLOW}[WARNING]${NC} 发现 $DEAD_CODE 个 dead_code 警告"
    cargo check 2>&1 | grep "dead_code" | head -10
    echo ""
    ISSUES=$((ISSUES + 1))
else
    echo -e "${GREEN}[OK]${NC} 无 dead_code 警告"
fi
cd "$PROJ_ROOT"

# --- 4. 检查过时的 Capability 注册 ---
echo ""
echo "--- [4/5] 检查 Capability 注册一致性 ---"

# 提取 registry 中注册的 capability ID
REGISTERED=$(grep -oP 'id\s*=\s*"[^"]+' "$SRC_DIR/extensions/" -r --include="*.rs" \
    | grep -oP '"[^"]+$' \
    | tr -d '"' \
    | sort -u || true)

# 检查每个注册的 capability 是否有对应的 handler 文件
for cap in $REGISTERED; do
    ext=$(echo "$cap" | cut -d. -f1)
    action=$(echo "$cap" | cut -d. -f2)

    # 检查 extension 目录是否存在
    if [ ! -d "$SRC_DIR/extensions/$ext" ] && [ "$ext" != "core" ] && [ "$ext" != "editor" ]; then
        echo -e "${RED}[ORPHAN]${NC} Capability '$cap' 的 extension 目录不存在: extensions/$ext/"
        ISSUES=$((ISSUES + 1))
    fi
done

# --- 5. 过时的 use/mod 语句 ---
echo ""
echo "--- [5/5] 检查过时的 mod/use 语句 ---"

# 检查 extensions/mod.rs 中引用的已删除模块
STALE_MODS=$(grep -n "mod " "$SRC_DIR/extensions/mod.rs" 2>/dev/null \
    | grep -E "(directory|terminal)" \
    || true)
if [ -n "$STALE_MODS" ]; then
    echo -e "${RED}[STALE]${NC} extensions/mod.rs 中引用已删除模块:"
    echo "$STALE_MODS"
    ISSUES=$((ISSUES + 1))
fi

# 检查 lib.rs 中引用的已删除命令
STALE_CMDS=$(grep -n "commands::" "$SRC_DIR/lib.rs" 2>/dev/null \
    | grep -E "(file::|checkout::|terminal)" \
    || true)
if [ -n "$STALE_CMDS" ]; then
    echo -e "${RED}[STALE]${NC} lib.rs 中引用已删除命令:"
    echo "$STALE_CMDS"
    ISSUES=$((ISSUES + 1))
fi

echo ""
echo "=========================================="
if [ $ISSUES -gt 0 ]; then
    echo -e "${RED}发现 $ISSUES 个问题需要清理${NC}"
    exit 1
else
    echo -e "${GREEN}扫描通过，未发现无用代码${NC}"
    exit 0
fi
