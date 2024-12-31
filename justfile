set windows-shell := ["pwsh.exe", "-NoLogo", "-Command"]

export RUST_BACKTRACE := "full"

clean:
    cargo clean

fmt:
    cargo +nightly fmt
    cargo +stable clippy
    prettier -w README.md
    prettier -w .github

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

jsonschema:
    cargo +stable run --bin wpmctl --locked -- schemagen >schema.unit.json

# this part is run in a nix shell because python is a nightmare
schemagen:
    rm -rf schema-docs
    mkdir -p schema-docs
    generate-schema-doc ./schema.unit.json --config template_name=js_offline --config minify=false ./schema-docs/
    mv ./schema-docs/schema.unit.html ./schema-docs/schema.html
