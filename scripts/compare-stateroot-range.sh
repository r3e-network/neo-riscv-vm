#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VM_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"
DEPLOY_DIR="${DEPLOY_DIR:-${VM_DIR}/mainnet-validation}"
HOST_LIB="${HOST_LIB:-${VM_DIR}/target/release/libneo_riscv_host.so}"
REFERENCE_RPC="${REFERENCE_RPC:-http://seed1.neo.org:10332}"
START_INDEX="${1:-1}"
END_INDEX="${2:-}"
LOG_FILE="${LOG_FILE:-${VM_DIR}/stateroot-compare.log}"

if ! command -v expect >/dev/null 2>&1; then
  echo "expect is required" >&2
  exit 1
fi

if [[ ! -f "${DEPLOY_DIR}/neo-cli.dll" ]]; then
  echo "neo-cli deploy not found: ${DEPLOY_DIR}/neo-cli.dll" >&2
  exit 1
fi

if [[ ! -f "${HOST_LIB}" ]]; then
  echo "host library not found: ${HOST_LIB}" >&2
  exit 1
fi

export VM_DIR DEPLOY_DIR HOST_LIB REFERENCE_RPC START_INDEX END_INDEX LOG_FILE

expect <<'EOF'
set timeout 30
set vm_dir $env(VM_DIR)
set deploy_dir $env(DEPLOY_DIR)
set host_lib $env(HOST_LIB)
set reference_rpc $env(REFERENCE_RPC)
set start_index $env(START_INDEX)
set end_index $env(END_INDEX)
set log_file $env(LOG_FILE)

proc query_reference_root {reference_rpc idx} {
    set payload [format {{"jsonrpc":"2.0","method":"getstateroot","params":[%s],"id":1}} $idx]
    set response [exec curl -s -X POST -H {Content-Type: application/json} -d $payload $reference_rpc]
    if {[regexp {"roothash":"(0x[0-9a-f]+)"} $response -> root]} {
        return $root
    }
    if {[regexp {"message":"([^"]+)"} $response -> message]} {
        return "ERROR:$message"
    }
    return "ERROR:unparseable"
}

set fh [open $log_file a]
puts $fh [format "\n%s start compare range %s..%s" [clock format [clock seconds] -format {%Y-%m-%d %H:%M:%S}] $start_index [expr {$end_index eq "" ? "auto" : $end_index}]]
flush $fh

cd $deploy_dir
set env(NEO_RISCV_HOST_LIB) $host_lib
spawn dotnet neo-cli.dll --noverify

expect {
    -re {neo> $} {}
    timeout {
        puts stderr "neo-cli prompt timeout"
        exit 1
    }
}

if {$end_index eq ""} {
    send -- "state height\r"
    expect {
        -re {LocalRootIndex:\s*([0-9]+)} { set end_index $expect_out(1,string) }
        timeout {
            puts stderr "failed to read LocalRootIndex"
            exit 1
        }
    }
    expect -re {neo> $}
}

for {set idx $start_index} {$idx <= $end_index} {incr idx} {
    send -- [format "state root --index %s\r" $idx]
    set local_root ""
    expect {
        -re {\"roothash\":\"(0x[0-9a-f]+)\"} { set local_root $expect_out(1,string) }
        -re {Warning: ([^\r\n]+)} { set local_root [format "WARN:%s" $expect_out(1,string)] }
        timeout {
            puts stderr [format "timeout reading local state root for block %s" $idx]
            puts $fh [format "%s TIMEOUT" $idx]
            flush $fh
            exit 1
        }
    }
    expect -re {neo> $}

    set reference_root [query_reference_root $reference_rpc $idx]
    puts $fh [format "%s %s %s" $idx $local_root $reference_root]
    flush $fh

    if {$local_root ne $reference_root} {
        puts stderr [format "MISMATCH %s local=%s reference=%s" $idx $local_root $reference_root]
        close $fh
        exit 2
    }
}

send -- "exit\r"
expect eof
close $fh
EOF
