#!/bin/bash
# =============================================================================
# GlanceMind API E2E Test Runner
# =============================================================================
# 运行完整的 E2E 测试套件
#
# 使用方法:
#   ./run_tests.sh              # 运行所有测试
#   ./run_tests.sh comments     # 仅运行 comments 测试
#   ./run_tests.sh auth         # 仅运行 auth 测试
#   ./run_tests.sh config       # 仅运行 config 测试
# =============================================================================

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# 颜色输出
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

echo_info() { echo -e "${GREEN}[INFO]${NC} $1"; }
echo_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
echo_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# 环境变量
export DATABASE_URL="${DATABASE_URL:-postgres://glancemind:testpassword@localhost:5434/glancemind_test}"
export API_BASE_URL="${API_BASE_URL:-http://localhost:8080}"
export E2E_NATS_URL="${E2E_NATS_URL:-nats://localhost:4223}"
export E2E_NATS_TOKEN="${E2E_NATS_TOKEN:-glancemind-dev-token}"

# 检查依赖
check_dependencies() {
    echo_info "检查依赖..."
    
    if ! command -v python3 &> /dev/null; then
        echo_error "Python3 未安装"
        exit 1
    fi
    
    if ! command -v docker &> /dev/null; then
        echo_error "Docker 未安装"
        exit 1
    fi
    
    # 安装 Python 依赖
    pip3 install -q -r requirements.txt
    echo_info "依赖检查完成"
}

# 启动测试环境
start_environment() {
    echo_info "启动测试环境..."
    
    docker compose -f docker-compose.e2e.yml up -d --build
    
    echo_info "等待数据库就绪..."
    for i in {1..30}; do
        if docker exec api-e2e-db pg_isready -U glancemind -d glancemind_test > /dev/null 2>&1; then
            echo_info "数据库就绪"
            break
        fi
        echo "  等待数据库... ($i/30)"
        sleep 2
    done
    
    # 检查 API 是否启动 (如果在 compose 中)
    if docker ps | grep -q api-e2e-server; then
        echo_info "等待 API 就绪..."
        for i in {1..30}; do
            if curl -sf http://localhost:8080/health > /dev/null 2>&1; then
                echo_info "API 就绪"
                break
            fi
            echo "  等待 API... ($i/30)"
            sleep 2
        done
    fi
}

# 停止测试环境
stop_environment() {
    echo_info "停止测试环境..."
    docker compose -f docker-compose.e2e.yml down -v 2>/dev/null || true
}

# 运行测试
run_tests() {
    local test_file="$1"
    
    echo ""
    echo "========================================"
    echo "GlanceMind API E2E Tests"
    echo "========================================"
    echo "Database: $DATABASE_URL"
    echo "API: $API_BASE_URL"
    echo "========================================"
    echo ""
    
    if [ -n "$test_file" ]; then
        echo_info "运行测试: $test_file"
        python3 -m pytest "test_${test_file}_api.py" -v --tb=short
    else
        echo_info "运行所有测试..."
        python3 -m pytest -v --tb=short
    fi
}

# 清理
cleanup() {
    echo_info "清理..."
    rm -rf __pycache__ .pytest_cache
}

# 主函数
main() {
    local test_target="$1"
    local skip_env="${SKIP_ENV:-false}"
    
    check_dependencies
    
    if [ "$skip_env" != "true" ]; then
        # 确保清理旧环境
        stop_environment
        start_environment
    fi
    
    # 运行测试
    local exit_code=0
    run_tests "$test_target" || exit_code=$?
    
    if [ "$skip_env" != "true" ]; then
        stop_environment
    fi
    
    cleanup
    
    echo ""
    if [ $exit_code -eq 0 ]; then
        echo_info "✓ 所有测试通过!"
    else
        echo_error "✗ 测试失败 (exit code: $exit_code)"
    fi
    
    exit $exit_code
}

# 处理中断信号
trap stop_environment EXIT

# 运行
main "$@"
