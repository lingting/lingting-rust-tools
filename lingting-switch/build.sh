#!/usr/bin/env bash
# 构建当前模块：调用仓库脚本 scripts/build_current.sh。
# 标准输出的最后一行为可执行文件的绝对路径。
set -euo pipefail

MODULE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$MODULE_DIR"

"$MODULE_DIR/../scripts/build_current.sh"
