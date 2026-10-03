#!/usr/bin/env bash
# 把调用模块编译为当前系统的可执行程序。
# 用法：在模块目录下执行本脚本，标准输出的最后一行为可执行文件的绝对路径。
set -euo pipefail

MODULE_DIR="$(pwd)"
MODULE="$(basename "$MODULE_DIR")"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT_DIR/target}"

if [[ ! -f "$MODULE_DIR/Cargo.toml" ]]; then
  echo "未找到 $MODULE_DIR/Cargo.toml，请在模块目录下调用本脚本" >&2
  exit 1
fi

echo "[构建] $MODULE" >&2
cargo build --release -p "$MODULE"

ARTIFACT="$TARGET_DIR/release/$MODULE"
if [[ -f "$ARTIFACT.exe" ]]; then
  ARTIFACT="$ARTIFACT.exe"
fi

if [[ ! -f "$ARTIFACT" ]]; then
  echo "未找到构建产物: $ARTIFACT" >&2
  exit 1
fi

echo "$ARTIFACT"
