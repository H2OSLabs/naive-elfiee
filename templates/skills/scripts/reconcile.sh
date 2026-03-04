#!/usr/bin/env bash
# reconcile.sh — 对比工作目录文件与 Elfiee block 记录
#
# 用法: bash reconcile.sh [project_path]
#
# 检测项：
# 有实际文件但 Elfiee 中无对应 document block
#
# 两种模式：
# - Git 项目：只检查有变更的文件（modified/new/staged）
# - 非 Git 项目：检查所有文件
#
# Agent 应在 elfiee_task_commit 前运行此脚本，补齐缺失的 block 记录。

set -euo pipefail

PROJECT="${1:-.}"
PROJECT=$(cd "$PROJECT" && pwd)

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
NC='\033[0m'

echo "=========================================="
echo "  Elfiee Reconciliation Check"
echo "  Project: $PROJECT"
echo "=========================================="
echo ""

# 检查 .elf/ 目录
if [ ! -d "$PROJECT/.elf" ]; then
    echo -e "${RED}[ERROR]${NC} No .elf/ directory found in $PROJECT"
    echo "  Run 'elf init' first."
    exit 1
fi

# 检查 elf CLI
if ! command -v elf &> /dev/null; then
    echo -e "${RED}[ERROR]${NC} 'elf' command not found in PATH"
    exit 1
fi

UNRECONCILED=0
RECONCILED=0

# --- 1. 获取需要检查的文件 ---
echo "--- [1/3] Scanning files ---"

cd "$PROJECT"

if git rev-parse --is-inside-work-tree &>/dev/null; then
    # Git 模式：只检查有变更的文件
    echo "  Mode: git (checking changed files only)"
    UNSTAGED=$(git diff --name-only 2>/dev/null || true)
    STAGED=$(git diff --cached --name-only 2>/dev/null || true)
    UNTRACKED=$(git ls-files --others --exclude-standard 2>/dev/null || true)
    CHECK_FILES=$(echo -e "${UNSTAGED}\n${STAGED}\n${UNTRACKED}" \
        | sort -u | grep -v '^$' || true)
else
    # 非 Git 模式：扫描所有文件
    echo "  Mode: full scan (no git, checking all files)"
    CHECK_FILES=$(find "$PROJECT" -type f \
        -not -path '*/.elf/*' \
        -not -path '*/.git/*' \
        -not -path '*/node_modules/*' \
        -not -path '*/__pycache__/*' \
        -not -path '*/.claude/*' \
        -not -name '.*' \
        | sed "s|^$PROJECT/||" \
        | sort || true)
fi

# 过滤：排除 .elf/ 和隐藏文件
CHECK_FILES=$(echo "$CHECK_FILES" | grep -v '^\.elf/' | grep -v '^\.' || true)

FILE_COUNT=$(echo "$CHECK_FILES" | grep -c . || echo 0)
echo "  Found $FILE_COUNT files to check"

if [ -z "$CHECK_FILES" ]; then
    echo -e "${GREEN}[OK]${NC} No files to check"
    exit 0
fi

# --- 2. 获取 Elfiee block 列表 ---
echo ""
echo "--- [2/3] Querying Elfiee blocks ---"

BLOCK_LIST=$(elf block list --project "$PROJECT" 2>/dev/null || true)
if [ -z "$BLOCK_LIST" ]; then
    echo -e "${YELLOW}[WARN]${NC} Could not retrieve block list (is the project initialized?)"
    UNRECONCILED=$FILE_COUNT
else
    echo "  Block list retrieved"
fi

# --- 3. 对比 ---
echo ""
echo "--- [3/3] Reconciliation ---"

while IFS= read -r file; do
    [ -z "$file" ] && continue

    filename=$(basename "$file")

    if echo "$BLOCK_LIST" | grep -q "$filename"; then
        echo -e "${GREEN}[OK]${NC} $file — has matching block"
        RECONCILED=$((RECONCILED + 1))
    else
        echo -e "${YELLOW}[MISSING]${NC} $file — no matching block found"
        echo "       → Sync: elf scan $file --project $PROJECT"
        echo "       → Link to task: elfiee_block_link(parent_id=TASK_ID, child_id=BLOCK_ID, relation=\"implement\")"
        UNRECONCILED=$((UNRECONCILED + 1))
    fi
done <<< "$CHECK_FILES"

# --- 汇总 ---
echo ""
echo "=========================================="
echo -e "  Results: ${GREEN}$RECONCILED reconciled${NC} / ${YELLOW}$UNRECONCILED unreconciled${NC}"
echo "=========================================="

if [ "$UNRECONCILED" -gt 0 ]; then
    echo ""
    echo "ACTION REQUIRED: Record unreconciled changes in Elfiee before elfiee_task_commit."
    exit 1
else
    echo ""
    echo "All files reconciled. Safe to elfiee_task_commit."
    exit 0
fi
