#!/usr/bin/env bash
# 把调用模块编译为当前系统的可执行程序。
# 用法：在模块目录下执行本脚本，标准输出的最后一行为可执行文件的绝对路径。
# 传 -d 时构建非 release 版本。
set -euo pipefail

DEBUG=0
while [[ $# -gt 0 ]]; do
  case "$1" in
    -d)
      DEBUG=1
      shift
      ;;
    *)
      echo "未知参数: $1" >&2
      exit 1
      ;;
  esac
done

MODULE_DIR="$(pwd)"
MODULE="$(basename "$MODULE_DIR")"
ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT_DIR/target}"

if [[ ! -f "$MODULE_DIR/Cargo.toml" ]]; then
  echo "未找到 $MODULE_DIR/Cargo.toml，请在模块目录下调用本脚本" >&2
  exit 1
fi

CARGO_BUILD_ARGS=(--release)
PROFILE_DIR="release"
if [[ "$DEBUG" -eq 1 ]]; then
  CARGO_BUILD_ARGS=()
  PROFILE_DIR="debug"
fi

echo "[构建] $MODULE" >&2
cargo build "${CARGO_BUILD_ARGS[@]}" -p "$MODULE"

ARTIFACT="$TARGET_DIR/$PROFILE_DIR/$MODULE"
if [[ -f "$ARTIFACT.exe" ]]; then
  ARTIFACT="$ARTIFACT.exe"
fi

if [[ ! -f "$ARTIFACT" ]]; then
  echo "未找到构建产物: $ARTIFACT" >&2
  exit 1
fi

echo "$ARTIFACT"
