set windows-shell := ["pwsh.exe", "-NoLogo", "-Command"]

export RUST_BACKTRACE := "full"

clean:
    cargo clean

fmt:
    cargo +nightly fmt
    cargo +stable clippy
    prettier -w README.md

install-targets *targets:
    "{{ targets }}" -split ' ' | ForEach-Object { just install-target $_ }

install-target target:
    cargo +stable install --path {{ target }} --locked

install:
    just install-targets wpmd wpmctl

run target:
    cargo +stable run --bin {{ target }} --locked

warn target $RUST_LOG="warn":
    just run {{ target }}

info target $RUST_LOG="info":
    just run {{ target }}

debug target $RUST_LOG="debug":
    just run {{ target }}

trace target $RUST_LOG="trace":
    just run {{ target }}