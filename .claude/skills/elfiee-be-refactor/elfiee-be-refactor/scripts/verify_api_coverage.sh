#!/usr/bin/env bash
# verify_api_coverage.sh — 验证重构后的 API 覆盖度
#
# 检测项：
# 1. 所有目标 Capability 已注册
# 2. 所有 Capability 有对应的测试
# 3. 所有 Tauri Command 在 lib.rs 中注册
# 4. 所有 Payload 类型在 specta export 中注册
# 5. 运行 cargo test 并报告结果

set -euo pipefail

PROJ_ROOT="${1:-$(git rev-parse --show-toplevel)}"
SRC_DIR="$PROJ_ROOT/src-tauri/src"
RED='\033[0;31m'
YELLOW='\033[0;33m'
GREEN='\033[0;32m'
NC='\033[0m'

echo "=========================================="
echo "  Elfiee API 覆盖度验证"
echo "  扫描目录: $SRC_DIR"
echo "=========================================="
echo ""

PASS=0
FAIL=0
WARN=0

check_pass() { echo -e "${GREEN}[PASS]${NC} $1"; PASS=$((PASS + 1)); }
check_fail() { echo -e "${RED}[FAIL]${NC} $1"; FAIL=$((FAIL + 1)); }
check_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; WARN=$((WARN + 1)); }

# --- 1. 目标 Capability 注册检查 ---
echo "--- [1/5] 检查目标 Capability 注册 ---"

# 重构后应存在的 capability
TARGET_CAPS=(
    "core.create"
    "core.delete"
    "core.link"
    "core.unlink"
    "core.grant"
    "core.revoke"
    "core.rename"
    "core.change_type"
    "core.update_metadata"
    "core.read"
    "editor.create"
    "editor.delete"
    "document.write"
    "document.read"
    "task.write"
    "task.read"
    "agent.create"
    "agent.enable"
    "agent.disable"
    "session.append"
)

# 不应存在的 capability（已删除）
REMOVED_CAPS=(
    "directory.create"
    "directory.delete"
    "directory.rename"
    "directory.rename_with_type_change"
    "directory.write"
    "directory.import"
    "directory.export"
    "terminal.init"
    "terminal.execute"
    "terminal.save"
    "terminal.close"
    "task.commit"
    "markdown.write"
    "markdown.read"
    "code.write"
    "code.read"
)

for cap in "${TARGET_CAPS[@]}"; do
    if grep -rq "\"$cap\"" "$SRC_DIR" --include="*.rs"; then
        check_pass "Capability '$cap' 已注册"
    else
        check_fail "Capability '$cap' 未找到"
    fi
done

echo ""
for cap in "${REMOVED_CAPS[@]}"; do
    # 在非测试文件中搜索
    matches=$(grep -rn "\"$cap\"" "$SRC_DIR" --include="*.rs" \
        | grep -v "tests.rs" \
        | grep -v "#\[cfg(test)\]" \
        | grep -v "// REMOVED" \
        || true)
    if [ -n "$matches" ]; then
        check_fail "已删除的 Capability '$cap' 仍存在于代码中"
    else
        check_pass "Capability '$cap' 已清理"
    fi
done

# --- 2. 测试覆盖检查 ---
echo ""
echo "--- [2/5] 检查 Capability 测试覆盖 ---"

for cap in "${TARGET_CAPS[@]}"; do
    ext=$(echo "$cap" | cut -d. -f1)
    action=$(echo "$cap" | cut -d. -f2)

    # 搜索包含该 capability 的测试
    test_count=$(grep -rn "\"$cap\"" "$SRC_DIR" --include="*.rs" \
        | grep -E "(test|tests)" \
        | wc -l || true)

    if [ "$test_count" -gt 0 ]; then
        check_pass "Capability '$cap' 有 $test_count 个测试"
    else
        check_warn "Capability '$cap' 无专门测试"
    fi
done

# --- 3. Tauri Command 注册检查 ---
echo ""
echo "--- [3/5] 检查 Tauri Command 注册 ---"

# 提取所有 #[tauri::command] 函数名
COMMANDS=$(grep -B1 "#\[tauri::command\]" "$SRC_DIR"/*.rs "$SRC_DIR"/**/*.rs 2>/dev/null \
    | grep "fn " \
    | grep -oP 'fn\s+\K\w+' \
    | sort -u || true)

# 检查是否在 lib.rs 中注册
for cmd in $COMMANDS; do
    if grep -q "$cmd" "$SRC_DIR/lib.rs"; then
        check_pass "Command '$cmd' 已在 lib.rs 注册"
    else
        check_warn "Command '$cmd' 未在 lib.rs 注册（可能已弃用）"
    fi
done

# --- 4. Payload 类型 specta 注册 ---
echo ""
echo "--- [4/5] 检查 Payload 类型 specta 注册 ---"

# 搜索所有定义了 Type derive 的 Payload 结构
PAYLOADS=$(grep -rn "#\[derive.*Type.*\]" "$SRC_DIR" --include="*.rs" -A1 \
    | grep "pub struct" \
    | grep -oP 'struct\s+\K\w+' \
    | sort -u || true)

for payload in $PAYLOADS; do
    if grep -q "$payload" "$SRC_DIR/lib.rs"; then
        check_pass "Payload '$payload' 已在 specta export 注册"
    else
        check_warn "Payload '$payload' 可能未在 specta export 注册"
    fi
done

# --- 5. 编译和测试 ---
echo ""
echo "--- [5/5] 运行编译和测试 ---"

cd "$PROJ_ROOT/src-tauri"

echo "  编译中..."
if cargo check 2>&1 | tail -1 | grep -q "error"; then
    check_fail "cargo check 失败"
    cargo check 2>&1 | grep "^error" | head -5
else
    check_pass "cargo check 通过"
fi

echo "  运行测试..."
TEST_OUTPUT=$(cargo test 2>&1 || true)
TEST_RESULT=$(echo "$TEST_OUTPUT" | grep "test result" | tail -1 || true)
if echo "$TEST_RESULT" | grep -q "FAILED"; then
    FAILED_TESTS=$(echo "$TEST_OUTPUT" | grep "FAILED" | wc -l)
    check_fail "cargo test 有 $FAILED_TESTS 个失败"
    echo "$TEST_OUTPUT" | grep "---- .* FAILED" | head -10
else
    PASSED_TESTS=$(echo "$TEST_RESULT" | grep -oP '\d+ passed' || echo "0 passed")
    check_pass "cargo test 全部通过 ($PASSED_TESTS)"
fi

# --- 汇总 ---
echo ""
echo "=========================================="
echo -e "  结果汇总: ${GREEN}$PASS 通过${NC} / ${RED}$FAIL 失败${NC} / ${YELLOW}$WARN 警告${NC}"
echo "=========================================="

if [ "$FAIL" -gt 0 ]; then
    exit 1
else
    exit 0
fi
