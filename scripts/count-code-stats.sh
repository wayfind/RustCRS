#!/bin/bash
# Code Statistics Generator for Claude Relay Service
# Generates comprehensive code metrics for CI reporting
# Excludes archived Node.js backend code from active counts

set -eu

# Get project root directory
PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$PROJECT_ROOT"

# Helper function to safely count files (using simple path-based find)
count_files_by_path() {
    local path_pattern="$1"
    shift
    if [ $# -eq 0 ]; then
        find "$path_pattern" -type f -name '*.rs' 2>/dev/null | wc -l | tr -d ' '
    else
        find "$path_pattern" -type f "$@" 2>/dev/null | wc -l | tr -d ' '
    fi
}

# Helper function to safely count LOC (using cat and wc -l for speed)
count_loc_by_path() {
    local path_pattern="$1"
    shift
    if [ $# -eq 0 ]; then
        find "$path_pattern" -type f -name '*.rs' -exec cat {} \; 2>/dev/null | wc -l
    else
        find "$path_pattern" -type f "$@" -exec cat {} \; 2>/dev/null | wc -l
    fi
}

# Helper function to get average LOC per file
calc_average() {
    local total_loc=$1
    local file_count=$2
    if [ "$file_count" -gt 0 ]; then
        awk "BEGIN {printf \"%.0f\", $total_loc / $file_count}"
    else
        echo "0"
    fi
}

# Helper function to calculate percentage
calc_percentage() {
    local part=$1
    local total=$2
    if [ "$total" -gt 0 ]; then
        awk "BEGIN {printf \"%.1f\", ($part / $total) * 100}"
    else
        echo "0.0"
    fi
}

echo "## 📊 Code Statistics Report"
echo ""

# ============================================
# SECTION 1: Active Codebase Statistics
# ============================================

# Rust Backend (excluding tests)
rust_backend_count=$(count_files_by_path "./rust/src" -name '*.rs')
rust_backend_loc=$(count_loc_by_path "./rust/src" -name '*.rs')

# Rust Tests
rust_test_count=$(count_files_by_path "./rust/tests" -name '*.rs')
rust_test_loc=$(count_loc_by_path "./rust/tests" -name '*.rs')

# Rust Benches
rust_bench_count=$(count_files_by_path "./rust/benches" -name '*.rs')
rust_bench_loc=$(count_loc_by_path "./rust/benches" -name '*.rs')

# Frontend (Vue/JS/TS in web/admin-spa/src)
frontend_count=$(count_files_by_path "./web/admin-spa/src" \( -name '*.vue' -o -name '*.js' -o -name '*.ts' -o -name '*.jsx' -o -name '*.tsx' \))
frontend_loc=$(count_loc_by_path "./web/admin-spa/src" \( -name '*.vue' -o -name '*.js' -o -name '*.ts' -o -name '*.jsx' -o -name '*.tsx' \))

# Configuration files (手写的配置，排除自动生成的 lock 文件和归档代码)
config_count=$(find . -type f \( -name 'Cargo.toml' -o -name '*.yml' -o -name '*.yaml' -o -name 'Makefile' -o -name '.prettierrc' -o -name '.eslintrc.cjs' -o -name 'package.json' \) ! -path '*/node_modules/*' ! -path '*/target/*' ! -path '*/nodejs-archive/*' ! -name 'package-lock.json' ! -name 'Cargo.lock' 2>/dev/null | wc -l | tr -d ' ')
config_loc=$(find . -type f \( -name 'Cargo.toml' -o -name '*.yml' -o -name '*.yaml' -o -name 'Makefile' -o -name '.prettierrc' -o -name '.eslintrc.cjs' -o -name 'package.json' \) ! -path '*/node_modules/*' ! -path '*/target/*' ! -path '*/nodejs-archive/*' ! -name 'package-lock.json' ! -name 'Cargo.lock' -exec cat {} \; 2>/dev/null | wc -l)

# Documentation (excluding node_modules, target, archived)
docs_count=$(find . -name '*.md' -type f ! -path '*/node_modules/*' ! -path '*/target/*' ! -path '*/nodejs-archive/*' 2>/dev/null | wc -l | tr -d ' ')
docs_loc=$(find . -name '*.md' -type f ! -path '*/node_modules/*' ! -path '*/target/*' ! -path '*/nodejs-archive/*' 2>/dev/null | xargs wc -l 2>/dev/null | tail -1 | awk '{print $1}' || echo "0")

# Scripts
scripts_count=$(count_files_by_path "./scripts" \( -name '*.sh' -o -name '*.js' \))
scripts_loc=$(count_loc_by_path "./scripts" \( -name '*.sh' -o -name '*.js' \))

# Calculate totals for active codebase
total_active_files=$((rust_backend_count + rust_test_count + rust_bench_count + frontend_count + config_count + scripts_count))
total_active_loc=$((rust_backend_loc + rust_test_loc + rust_bench_loc + frontend_loc + config_loc + scripts_loc))

# ============================================
# SECTION 2: Summary Table
# ============================================

echo "### 📈 Summary"
echo ""
echo "| Metric | Value |"
echo "|--------|-------|"
echo "| **Total Active Files** | $total_active_files files |"
echo "| **Total Active LOC** | $(printf "%'d" $total_active_loc) lines |"
echo "| **Rust Backend** | $rust_backend_count files ($(printf "%'d" $rust_backend_loc) LOC) |"
echo "| **Rust Tests** | $rust_test_count files ($(printf "%'d" $rust_test_loc) LOC) |"
echo "| **Frontend (Vue/JS/TS)** | $frontend_count files ($(printf "%'d" $frontend_loc) LOC) |"
echo "| **Configuration** | $config_count files ($(printf "%'d" $config_loc) LOC) |"
echo "| **Documentation** | $docs_count files ($(printf "%'d" $docs_loc) LOC) |"
echo ""

# ============================================
# SECTION 4: Detailed Breakdown
# ============================================

echo "### 📊 Code Distribution"
echo ""
echo "| Category | Files | Lines | % of Total | Avg Lines/File |"
echo "|----------|-------|-------|------------|----------------|"

# Rust Backend
rust_backend_pct=$(calc_percentage $rust_backend_loc $total_active_loc)
rust_backend_avg=$(calc_average $rust_backend_loc $rust_backend_count)
echo "| **Rust Backend** | $rust_backend_count | $(printf "%'d" $rust_backend_loc) | ${rust_backend_pct}% | $rust_backend_avg |"

# Rust Tests
rust_test_pct=$(calc_percentage $rust_test_loc $total_active_loc)
rust_test_avg=$(calc_average $rust_test_loc $rust_test_count)
echo "| **Rust Tests** | $rust_test_count | $(printf "%'d" $rust_test_loc) | ${rust_test_pct}% | $rust_test_avg |"

# Rust Benches
if [ "$rust_bench_count" -gt 0 ]; then
    rust_bench_pct=$(calc_percentage $rust_bench_loc $total_active_loc)
    rust_bench_avg=$(calc_average $rust_bench_loc $rust_bench_count)
    echo "| **Rust Benchmarks** | $rust_bench_count | $(printf "%'d" $rust_bench_loc) | ${rust_bench_pct}% | $rust_bench_avg |"
fi

# Frontend
frontend_pct=$(calc_percentage $frontend_loc $total_active_loc)
frontend_avg=$(calc_average $frontend_loc $frontend_count)
echo "| **Frontend** | $frontend_count | $(printf "%'d" $frontend_loc) | ${frontend_pct}% | $frontend_avg |"

# Config
config_pct=$(calc_percentage $config_loc $total_active_loc)
config_avg=$(calc_average $config_loc $config_count)
echo "| **Configuration** | $config_count | $(printf "%'d" $config_loc) | ${config_pct}% | $config_avg |"

# Scripts
if [ "$scripts_count" -gt 0 ]; then
    scripts_pct=$(calc_percentage $scripts_loc $total_active_loc)
    scripts_avg=$(calc_average $scripts_loc $scripts_count)
    echo "| **Scripts** | $scripts_count | $(printf "%'d" $scripts_loc) | ${scripts_pct}% | $scripts_avg |"
fi

echo ""

# ============================================
# SECTION 5: Top 5 Largest Files
# ============================================

echo "### 🔝 Top 5 Largest Files"
echo ""

# Top Rust Backend files
echo "#### Rust Backend"
if [ "$rust_backend_count" -gt 0 ]; then
    find ./rust/src -name '*.rs' -type f -exec wc -l {} + 2>/dev/null | sort -rn | head -5 | while read lines file; do
        [ "$file" = "total" ] && continue
        rel_path="${file#./}"
        echo "- \`$rel_path\` ($lines lines)"
    done
else
    echo "- No Rust backend files found"
fi
echo ""

# Top Frontend files
echo "#### Frontend"
if [ "$frontend_count" -gt 0 ]; then
    find ./web/admin-spa/src -type f \( -name '*.vue' -o -name '*.js' -o -name '*.ts' \) -exec wc -l {} + 2>/dev/null | sort -rn | head -5 | while read lines file; do
        [ "$file" = "total" ] && continue
        rel_path="${file#./}"
        echo "- \`$rel_path\` ($lines lines)"
    done
else
    echo "- No frontend files found"
fi
echo ""

# Top Test files
if [ "$rust_test_count" -gt 0 ]; then
    echo "#### Rust Tests"
    find ./rust/tests -name '*.rs' -type f -exec wc -l {} + 2>/dev/null | sort -rn | head -5 | while read lines file; do
        [ "$file" = "total" ] && continue
        rel_path="${file#./}"
        echo "- \`$rel_path\` ($lines lines)"
    done
    echo ""
fi

# ============================================
# Footer
# ============================================

echo "---"
echo "*Generated on $(date -u +"%Y-%m-%d %H:%M:%S UTC") • Auto-updated on every PR*"
