#!/usr/bin/env bash

resolve_host_lib() {
  local root_dir="$1"
  local mode="${2:-release}"
  local target_dir="${root_dir}/target/${mode}"
  local os_name
  os_name="$(uname -s)"

  local candidates=()
  case "${os_name}" in
    Darwin)
      candidates+=("${target_dir}/libneo_riscv_host.dylib")
      ;;
    Linux)
      candidates+=("${target_dir}/libneo_riscv_host.so")
      ;;
    MINGW*|MSYS*|CYGWIN*)
      candidates+=("${target_dir}/neo_riscv_host.dll")
      ;;
  esac

  candidates+=(
    "${target_dir}/libneo_riscv_host.so"
    "${target_dir}/libneo_riscv_host.dylib"
    "${target_dir}/neo_riscv_host.dll"
  )

  local candidate
  for candidate in "${candidates[@]}"; do
    if [[ -f "${candidate}" ]]; then
      printf '%s\n' "${candidate}"
      return 0
    fi
  done

  case "${os_name}" in
    Darwin)
      printf '%s\n' "${target_dir}/libneo_riscv_host.dylib"
      ;;
    MINGW*|MSYS*|CYGWIN*)
      printf '%s\n' "${target_dir}/neo_riscv_host.dll"
      ;;
    *)
      printf '%s\n' "${target_dir}/libneo_riscv_host.so"
      ;;
  esac
}

if [[ "${BASH_SOURCE[0]}" == "$0" ]]; then
  resolve_host_lib "${1:-$(pwd)}" "${2:-release}"
fi
