#!/usr/bin/env bash
# scan_unused_deps.sh — 扫描 Cargo.toml 中可能未使用的依赖
#
# 原理：检查每个依赖的 crate 名是否在源码中被 use/extern crate 引用
# 注意：某些依赖可能通过 proc macro 或 feature flag 隐式使用，需人工确认

set -euo pipefail

PROJ_ROOT="${1:-$(git rev-parse --show-toplevel)}"
CARGO_TOML="$PROJ_ROOT/src-tauri/Cargo.toml"
SRC_DIR="$PROJ_ROOT/src-tauri/src"

RED='\033[0;31m'
YELLOW='\033[0;33m'
GREEN='\033[0;32m'
NC='\033[0m'

echo "=========================================="
echo "  Elfiee 未使用依赖扫描"
echo "  Cargo.toml: $CARGO_TOML"
echo "=========================================="
echo ""

# 提取 [dependencies] 中的 crate 名称
# 处理多种格式：name = "version", name = { version = "..." }, name.workspace = true
DEPS=$(grep -E '^\s*[a-z]' "$CARGO_TOML" \
    | sed -n '/^\[dependencies\]/,/^\[/p' \
    | grep -v '^\[' \
    | grep -v '^#' \
    | grep -v '^\s*$' \
    | awk -F'[= ]' '{print $1}' \
    | tr -d ' ' \
    | sort -u)

# 重构后应该移除的依赖（根据 migration 文档）
SHOULD_REMOVE=(
    "portable-pty:Terminal Extension 已删除"
    "base64:Terminal 输出编码已不需要"
    "zip:.elf 从 ZIP 改为目录格式"
    "walkdir:Directory Extension 已删除"
    "ignore:Directory Extension 已删除"
)

echo "--- [1/3] 检查重构后应移除的依赖 ---"
for entry in "${SHOULD_REMOVE[@]}"; do
    dep=$(echo "$entry" | cut -d: -f1)
    reason=$(echo "$entry" | cut -d: -f2)
    if grep -q "^$dep" "$CARGO_TOML" 2>/dev/null || grep -q "\"$dep\"" "$CARGO_TOML" 2>/dev/null; then
        # 将 crate 名中的 - 转为 _ 用于源码搜索
        crate_use=$(echo "$dep" | tr '-' '_')
        usage_count=$(grep -rn "$crate_use" "$SRC_DIR" --include="*.rs" | grep -v "target/" | wc -l || true)
        if [ "$usage_count" -gt 0 ]; then
            echo -e "${YELLOW}[IN USE]${NC} '$dep' ($reason) — 仍有 $usage_count 处引用，需先清理代码"
        else
            echo -e "${RED}[REMOVE]${NC} '$dep' ($reason) — 源码中无引用，可安全移除"
        fi
    else
        echo -e "${GREEN}[DONE]${NC} '$dep' 已移除"
    fi
done

echo ""
echo "--- [2/3] 扫描可能未使用的其他依赖 ---"
SUSPICIOUS=0
for dep in $DEPS; do
    # 跳过 path 依赖（内部 crate）
    if grep -A2 "^$dep" "$CARGO_TOML" | grep -q "path ="; then
        continue
    fi

    # 将 - 转为 _ 用于源码搜索（Rust crate 命名惯例）
    crate_use=$(echo "$dep" | tr '-' '_')

    # 搜索源码中的使用
    usage=$(grep -rn "use ${crate_use}" "$SRC_DIR" --include="*.rs" 2>/dev/null | wc -l || true)
    usage2=$(grep -rn "${crate_use}::" "$SRC_DIR" --include="*.rs" 2>/dev/null | wc -l || true)
    # 有些 crate 通过 proc macro 使用，搜索 derive/attribute
    usage3=$(grep -rn "#\[.*${crate_use}" "$SRC_DIR" --include="*.rs" 2>/dev/null | wc -l || true)

    total=$((usage + usage2 + usage3))

    if [ "$total" -eq 0 ]; then
        # 某些 crate 有别名或隐式使用
        case "$dep" in
            tauri|tauri-plugin-*|tauri-specta)
                continue  # Tauri 生态隐式使用
                ;;
            serde|serde_json|specta|specta-typescript)
                continue  # 序列化/类型生成，通过 derive 使用
                ;;
            async-trait)
                continue  # proc macro
                ;;
            *)
                echo -e "${YELLOW}[SUSPICIOUS]${NC} '$dep' — 源码中未发现直接引用（可能通过宏隐式使用）"
                SUSPICIOUS=$((SUSPICIOUS + 1))
                ;;
        esac
    fi
done

if [ "$SUSPICIOUS" -eq 0 ]; then
    echo -e "${GREEN}[OK]${NC} 未发现可疑的未使用依赖"
fi

echo ""
echo "--- [3/3] 运行 cargo check 验证依赖可编译 ---"
cd "$PROJ_ROOT/src-tauri"
if cargo check 2>&1 | grep -q "error"; then
    echo -e "${RED}[ERROR]${NC} cargo check 失败，请先修复编译错误"
    cargo check 2>&1 | grep "error" | head -5
else
    echo -e "${GREEN}[OK]${NC} cargo check 通过"
fi

echo ""
echo "=========================================="
echo "提示：标记为 [SUSPICIOUS] 的依赖需要人工确认是否可移除"
echo "      标记为 [REMOVE] 的依赖可安全从 Cargo.toml 删除"
echo "=========================================="
