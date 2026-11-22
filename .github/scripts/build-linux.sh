#!/usr/bin/env bash
set -euo pipefail

platform="${1:?Usage: build-linux.sh <distribution-version-architecture>}"
rust_toolchain="${RUST_TOOLCHAIN:-stable}"
artifact_dir="${ARTIFACT_DIR:-$PWD/dist}"
target_dir="${CARGO_TARGET_DIR:-$PWD/target}"
build_tmp="$(mktemp -d)"
trap 'rm -rf "$build_tmp"' EXIT

export PATH="$HOME/.cargo/bin:$PATH"
if ! command -v rustup >/dev/null 2>&1; then
    curl --proto '=https' --tlsv1.2 --fail --silent --show-error --location \
        https://sh.rustup.rs --output "$build_tmp/rustup-init.sh"
    sh "$build_tmp/rustup-init.sh" -y --profile minimal --default-toolchain "$rust_toolchain"
fi
rustup toolchain install "$rust_toolchain" --profile minimal
cargo +"$rust_toolchain" --version
rustc +"$rust_toolchain" --version

cargo +"$rust_toolchain" test --locked
cargo +"$rust_toolchain" build --locked --release --bins

# Run the binaries in their target distribution before publishing artifacts.
"$target_dir/release/texted" --version
"$target_dir/release/texted-tool" --version
mkdir "$build_tmp/site"
"$target_dir/release/texted-tool" bootstrap --out-dir "$build_tmp/site"
test -f "$build_tmp/site/texted.toml"
test -f "$build_tmp/site/template/page.tpl"

version="$(cargo +"$rust_toolchain" pkgid --locked)"
version="${version##*#}"
version="${version##*@}"
package="texted-${version}-${platform}"
mkdir -p "$artifact_dir" "$build_tmp/$package"
cp "$target_dir/release/texted" "$target_dir/release/texted-tool" "$build_tmp/$package/"
cp README.md LICENSE ChangeLog texted.toml "$build_tmp/$package/"
cp -R res "$build_tmp/$package/"
tar -czf "$artifact_dir/$package.tar.gz" -C "$build_tmp" "$package"
