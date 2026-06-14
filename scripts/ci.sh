#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
work="$(mktemp -d "${TMPDIR:-/tmp}/rustify-ci-XXXXXX")"
playground_pid=""
cleanup() {
  if [[ -n "$playground_pid" ]]; then
    kill "$playground_pid" 2>/dev/null || true
  fi
  rm -rf "$work"
}
trap cleanup EXIT

cd "$root"

cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace

npm ci --prefix packages/eslint-plugin-rustify
npm test --prefix packages/eslint-plugin-rustify
npm ci --prefix packages/vscode-rustify
npm test --prefix packages/vscode-rustify
npm run test-e2e --prefix packages/vscode-rustify

examples=(
  examples/greet.ts
  examples/control-flow.ts
  examples/mvp-types.ts
  examples/objects.ts
  examples/operations.ts
  examples/collections.ts
  examples/runtime.ts
  examples/async.ts
  examples/async-runtime.ts
  examples/modules/main.ts
)

for example in "${examples[@]}"; do
  name="$(basename "${example%.ts}")"
  output="$work/$name"
  cargo run -q -p rustify-cli -- check "$example"
  cargo run -q -p rustify-cli -- compile "$example" --out "$output" --cargo
  RUSTFLAGS="-D warnings" cargo check --manifest-path "$output/Cargo.toml"
done

if cargo run -q -p rustify-cli -- check examples/invalid.ts; then
  echo "examples/invalid.ts unexpectedly passed native validation" >&2
  exit 1
fi

hybrid="$work/hybrid-project"
mkdir -p "$hybrid/src"
cp examples/hybrid.ts "$hybrid/src/main.ts"
cat >"$hybrid/rustify.json" <<'JSON'
{
  "entry": "src/main.ts",
  "out": "dist",
  "cargo": true,
  "package_name": "rustify-hybrid-ci",
  "mode": "hybrid"
}
JSON

(
  cd "$hybrid"
  cargo run -q --manifest-path "$root/Cargo.toml" -p rustify-cli -- compile
  test -f dist/rustify-hybrid.json
  test -f dist/fallback/src/main.ts
  npm run --silent --prefix dist start | grep -F "Hello from the V8 fallback"
)

RUSTIFY_PLAYGROUND_PORT=39001 cargo run -q -p rustify-playground >"$work/playground.log" 2>&1 &
playground_pid="$!"
for _ in {1..30}; do
  if curl -fsS http://127.0.0.1:39001/api/example >/dev/null 2>&1; then
    break
  fi
  sleep 1
done
curl -fsS \
  -H "content-type: application/json" \
  --data '{"source":"function greet(name: string): string { return `Hi ${name}` }"}' \
  http://127.0.0.1:39001/api/compile | grep -F '"valid":true'
curl -fsS \
  -H "content-type: application/json" \
  --data '{"source":"function unsafe(value: any): void {}"}' \
  http://127.0.0.1:39001/api/compile | grep -F '"code":"SFT001"'
kill "$playground_pid"
wait "$playground_pid" 2>/dev/null || true
playground_pid=""

echo "Rustify CI integration gate passed."
