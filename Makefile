.PHONY: build test perf perf_history test_coverage setup check check_fix check_audit check-commit-msg

build:
	cargo build --release

test:
	@set -eu; \
	out="$$(mktemp)"; \
	if cargo test --quiet >"$$out" 2>&1; then \
		awk 'BEGIN { passed=0; failed=0; ignored=0; measured=0; filtered=0; suites=0 } \
			/test result:/ { \
				suites++; \
				for (i = 1; i <= NF; i++) { \
					if ($$i == "passed;") passed += $$(i - 1); \
					else if ($$i == "failed;") failed += $$(i - 1); \
					else if ($$i == "ignored;") ignored += $$(i - 1); \
					else if ($$i == "measured;") measured += $$(i - 1); \
					else if ($$i == "filtered") filtered += $$(i - 1); \
				} \
			} \
			END { \
				if (suites == 0) { \
					print "Test totals: 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out"; \
				} else { \
					printf "Test totals: %d passed; %d failed; %d ignored; %d measured; %d filtered out\n", passed, failed, ignored, measured, filtered; \
				} \
			}' "$$out"; \
	else \
		cat "$$out"; \
		rm -f "$$out"; \
		exit 1; \
	fi; \
	rm -f "$$out"

check:
	@cargo fmt --all -- --check

check_fix:
	@cargo fmt --all
	@cargo clippy --quiet --all-targets --all-features -- -D warnings

check_audit:
	cargo fmt --all -- --check
	cargo clippy --quiet --all-targets --all-features -- -D warnings
	cargo machete
	cargo +nightly udeps --all-targets --all-features

perf: build
	@echo "── lint: 5000 files, 21 lang types ──"
	@cargo bench --bench lint_bench 2>&1 | grep -E "^[a-z_]|time:"
	@echo ""
	@echo "── check: 5000 files, directive validation ──"
	@cargo bench --bench scan_bench 2>&1 | grep -E "^[a-z_]|time:"
	@echo ""
	@echo "── parser: single 16k-line diff ──"
	@cargo bench --bench latency_16kloc_bench 2>&1 | grep -E "^[a-z_]|time:"

perf_history:
	@gh api 'repos/slnc/ifchange/contents/bench/data.js?ref=gh-pages' --jq '.content' \
		| base64 -d \
		| sed 's/^window.BENCHMARK_DATA = //' \
		| jq -r '.entries.Benchmark[-100:] | reverse | .[] | (.commit.timestamp | split("T")[0]) as $$d | (.commit.id[:7]) as $$s | (.benches | map({(.name): (.value / 1e6 | . * 100 | round / 100 | tostring + " ms")}) | add) as $$b | [$$d, $$s, $$b["lint_latency_16kloc_diff"] // "-", $$b["lint_1000_files"] // "-", $$b["lint_5000_files"] // "-", $$b["scan_5000_files"] // "-"] | @tsv' \
		| (echo "DATE\tCOMMIT\tLINT_16K\tLINT_1K\tLINT_5K\tSCAN_5K" && cat) \
		| column -t -s '	'

test_coverage:
	cargo llvm-cov --workspace --all-features --html
	cargo llvm-cov report --json --summary-only --output-path target/llvm-cov/summary.json
	@jq -r '.data[0].totals as $$t | "Coverage: lines \($$t.lines.covered)/\($$t.lines.count) (\(($$t.lines.percent*100|round)/100)%) | regions \($$t.regions.covered)/\($$t.regions.count) (\(($$t.regions.percent*100|round)/100)%) | functions \($$t.functions.covered)/\($$t.functions.count) (\(($$t.functions.percent*100|round)/100)%)"' target/llvm-cov/summary.json

setup:
	@set -eu; \
	os="$$(uname -s)"; \
	case "$$os" in \
		Linux|Darwin) ;; \
		*) echo "Unsupported OS: $$os (supported: Linux, macOS)"; exit 1 ;; \
	esac; \
	git config core.hooksPath .githooks; \
	echo "Git hooks path set to .githooks"; \
	echo "Conventional commit validation is now active via commit-msg hook."; \
	install_steps=""; \
	missing_tools=""; \
	pm_install=""; \
	add_missing() { \
		missing_tools="$$missing_tools $$1"; \
	}; \
	add_step() { \
		install_steps="$${install_steps}$$1\n"; \
	}; \
	if [ "$$os" = "Darwin" ]; then \
		if command -v brew >/dev/null 2>&1; then \
			pm_install="brew install"; \
		else \
			echo "Homebrew is required on macOS to auto-install dependencies."; \
			echo "Install Homebrew first: https://brew.sh"; \
			exit 1; \
		fi; \
	else \
		if command -v apt-get >/dev/null 2>&1; then \
			pm_install="sudo apt-get update && sudo apt-get install -y"; \
		elif command -v dnf >/dev/null 2>&1; then \
			pm_install="sudo dnf install -y"; \
		elif command -v yum >/dev/null 2>&1; then \
			pm_install="sudo yum install -y"; \
		elif command -v pacman >/dev/null 2>&1; then \
			pm_install="sudo pacman -Sy --needed"; \
		elif command -v zypper >/dev/null 2>&1; then \
			pm_install="sudo zypper install -y"; \
		else \
			echo "No supported Linux package manager found (apt/dnf/yum/pacman/zypper)."; \
			exit 1; \
		fi; \
	fi; \
	if ! command -v jq >/dev/null 2>&1; then \
		add_missing jq; \
		add_step "$$pm_install jq"; \
	fi; \
	if ! command -v rustc >/dev/null 2>&1 || ! command -v cargo >/dev/null 2>&1; then \
		add_missing "rust (rustc/cargo)"; \
		if [ "$$os" = "Darwin" ]; then \
			add_step "brew install rustup-init"; \
			add_step "rustup-init -y"; \
		else \
			if ! command -v curl >/dev/null 2>&1; then \
				add_missing curl; \
				add_step "$$pm_install curl"; \
			fi; \
			add_step "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y"; \
		fi; \
		add_step ". \"$$HOME/.cargo/env\""; \
	fi; \
	if ! command -v rustup >/dev/null 2>&1; then \
		add_missing rustup; \
		if [ "$$os" = "Darwin" ]; then \
			add_step "brew install rustup-init"; \
			add_step "rustup-init -y"; \
		else \
			if ! command -v curl >/dev/null 2>&1; then \
				add_missing curl; \
				add_step "$$pm_install curl"; \
			fi; \
			add_step "curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y"; \
		fi; \
		add_step ". \"$$HOME/.cargo/env\""; \
	fi; \
	if command -v cargo >/dev/null 2>&1; then \
		if ! cargo llvm-cov --version >/dev/null 2>&1; then \
			add_missing cargo-llvm-cov; \
			add_step "rustup component add llvm-tools-preview"; \
			add_step "cargo install cargo-llvm-cov"; \
		fi; \
		if ! cargo machete --version >/dev/null 2>&1; then \
			add_missing cargo-machete; \
			add_step "cargo install --locked cargo-machete"; \
		fi; \
		if ! cargo udeps --version >/dev/null 2>&1; then \
			add_missing cargo-udeps; \
			add_step "cargo install --locked cargo-udeps"; \
		fi; \
	else \
		add_missing cargo-llvm-cov; \
		add_step "rustup component add llvm-tools-preview"; \
		add_step "cargo install cargo-llvm-cov"; \
		add_missing cargo-machete; \
		add_step "cargo install --locked cargo-machete"; \
		add_missing cargo-udeps; \
		add_step "cargo install --locked cargo-udeps"; \
	fi; \
	if command -v rustup >/dev/null 2>&1; then \
		if ! rustup toolchain list | grep -Eq '^nightly'; then \
			add_missing "rustup toolchain nightly"; \
			add_step "rustup toolchain install nightly"; \
		fi; \
		if ! rustup component list --toolchain nightly | grep -Eq '^rust-src \(installed\)'; then \
			add_missing "nightly rust-src"; \
			add_step "rustup component add rust-src --toolchain nightly"; \
		fi; \
		if ! rustup component list --toolchain nightly | grep -Eq '^llvm-tools-preview \(installed\)'; then \
			add_missing "nightly llvm-tools-preview"; \
			add_step "rustup component add llvm-tools-preview --toolchain nightly"; \
		fi; \
	fi; \
	if [ -z "$$missing_tools" ]; then \
		echo "All development tools are already installed."; \
		echo "Available: jq, rustc, cargo, rustup, cargo-llvm-cov, cargo-machete, cargo-udeps, nightly+rcomponents"; \
		exit 0; \
	fi; \
	echo "Missing tools:$$missing_tools"; \
	echo ""; \
	echo "Commands that will be run:"; \
	printf "%b" "$$install_steps" | sed 's/^/  /'; \
	echo ""; \
	printf "Run these commands now? [y/N] "; \
	read -r reply; \
	case "$$reply" in \
		y|Y|yes|YES) \
			printf "%b" "$$install_steps" | while IFS= read -r cmd; do \
				[ -n "$$cmd" ] || continue; \
				echo "+ $$cmd"; \
				sh -c "$$cmd"; \
			done; \
			echo "Setup complete."; \
			;; \
		*) \
			echo "Skipped installation."; \
			;; \
	esac

check-commit-msg:
	@if [ -z "$$MSG_FILE" ]; then \
		echo "Usage: make check-commit-msg MSG_FILE=.git/COMMIT_EDITMSG"; \
		exit 1; \
	fi
	@.githooks/commit-msg "$$MSG_FILE"
