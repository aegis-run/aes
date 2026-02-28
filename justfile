set shell := ["bash", "-euo", "pipefail", "-c"]
set dotenv-load := true

llvm_cov_output := "target/coverage"

[private]
default:
    @just --list --unsorted

# ------------------------------------------------------------------ #
# Development                                                          #
# ------------------------------------------------------------------ #

# Format all code
[group("development")]
fmt:
    cargo fmt --all

# Check formatting without modifying files
[group("development")]
fmt-check:
    cargo fmt --all -- --check

# Run clippy, deny warnings
[group("development")]
lint:
    cargo clippy --all-targets --all-features -- -D warnings

# Run all checks (fmt + lint + deny)
[group("development")]
check: fmt-check lint deny

# Run tests with nextest
[group("development")]
test *args:
    cargo nextest run --all-features {{ args }}

# Watch for changes and re-run tests
[group("development")]
watch:
    cargo watch -x "nextest run --all-features"

# ------------------------------------------------------------------ #
# Snapshots (insta)                                                    #
# ------------------------------------------------------------------ #

# Review pending insta snapshots
[group("snapshot")]
snap-review:
    cargo insta review

# Accept all pending snapshots (use with caution)
[group("snapshot")]
snap-accept:
    cargo insta accept

# Reject all pending snapshots
[group("snapshot")]
snap-reject:
    cargo insta reject

# Run tests and open snapshot review immediately if any changed
[group("snapshot")]
snap-test:
    cargo insta test --review

# ------------------------------------------------------------------ #
# Coverage                                                             #
# ------------------------------------------------------------------ #

# Generate coverage report and open in browser
[group("coverage")]
cov:
    cargo llvm-cov nextest --all-features --open

# Generate lcov report (used in CI)
[group("coverage")]
cov-lcov:
    mkdir -p {{ llvm_cov_output }}
    cargo llvm-cov nextest --all-features --lcov --output-path {{ llvm_cov_output }}/lcov.info
    @echo "lcov report written to {{ llvm_cov_output }}/lcov.info"

# Generate HTML report without opening
[group("coverage")]
cov-html:
    mkdir -p {{ llvm_cov_output }}
    cargo llvm-cov nextest --all-features --html --output-dir {{ llvm_cov_output }}/html
    @echo "HTML report written to {{ llvm_cov_output }}/html/index.html"

# ------------------------------------------------------------------ #
# Dependency management                                                #
# ------------------------------------------------------------------ #

# Run cargo-deny checks (licenses + advisories + bans)
[group("dependency management")]
deny:
    cargo deny check

# Run only advisory checks
[group("dependency management")]
deny-advisories:
    cargo deny check advisories

# Run cargo audit (alternative/complementary to deny)
[group("dependency management")]
audit:
    cargo audit

# Update dependencies
[group("dependency management")]
update:
    cargo update

# Show outdated dependencies
[group("dependency management")]
outdated:
    cargo outdated

# ------------------------------------------------------------------ #
# Build                                                                #
# ------------------------------------------------------------------ #

# Debug build
[group("build")]
build:
    cargo build --all-features

# Release build
[group("build")]
build-release:
    cargo build --release --all-features

# Build via nix (reproducible)
[group("build")]
nix-build:
    nix build
    @echo "binary at ./result/bin/aes"

# Check the flake
[group("build")]
nix-check:
    nix flake check

# Update flake inputs
[group("build")]
nix-update:
    nix flake update

# ------------------------------------------------------------------ #
# Release                                                              #
# ------------------------------------------------------------------ #

# Verify that Cargo.toml version matches a given tag
[group("release")]
verify-version tag:
    #!/usr/bin/env bash
    CARGO_VERSION=$(cargo metadata --no-deps --format-version 1 | jq -r '.packages[0].version')
    TAG_VERSION="{{ tag }}"
    TAG_VERSION="${TAG_VERSION#v}"
    if [ "$CARGO_VERSION" != "$TAG_VERSION" ]; then
      echo "❌ Version mismatch: Cargo.toml=$CARGO_VERSION tag=$TAG_VERSION"
      exit 1
    fi
    echo "✓ Version $CARGO_VERSION matches tag {{ tag }}"

# Tag and push a release (e.g. just release 0.2.0)
[group("release")]
release version:
    @just verify-version {{ version }}
    git tag -a "v{{ version }}" -m "Release v{{ version }}"
    git push origin "v{{ version }}"
    @echo "✓ Tagged and pushed v{{ version }}"

# ------------------------------------------------------------------ #
# CI (mirrors what GitHub Actions runs)                                #
# ------------------------------------------------------------------ #

# Run the full CI pipeline locally
ci: check test snap-test cov-lcov deny audit
    @echo "✓ CI pipeline passed"

# ------------------------------------------------------------------ #
# Housekeeping                                                         #
# ------------------------------------------------------------------ #

# Remove build artifacts
[group("misc")]
clean:
    cargo clean
    rm -rf {{ llvm_cov_output }}

# Remove fuzz artifacts and corpus
[group("misc")]
clean-fuzz:
    rm -rf fuzz/target fuzz/corpus fuzz/artifacts

# Remove everything
[group("misc")]
clean-all: clean clean-fuzz

# Print tool versions (useful for debugging CI vs local discrepancies)
versions:
    @echo "rustc:          $(rustc --version)"
    @echo "cargo:          $(cargo --version)"
    @echo "rustfmt:        $(rustfmt --version)"
    @echo "clippy:         $(cargo clippy --version)"
    @echo "nextest:        $(cargo nextest --version)"
    @echo "llvm-cov:       $(cargo llvm-cov --version)"
    @echo "cargo-deny:     $(cargo deny --version)"
    @echo "cargo-insta:    $(cargo insta --version)"
    @echo "just:           $(just --version)"
