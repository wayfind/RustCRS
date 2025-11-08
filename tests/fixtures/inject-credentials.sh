#!/bin/bash
###############################################################################
# Credentials Injection Script for E2E Testing
#
# Purpose: Inject real, human-verified credentials for automated E2E testing
#
# Usage:
#   1. Copy credentials.json.example to credentials.json
#   2. Fill in your real credentials
#   3. Run: bash tests/fixtures/inject-credentials.sh
#
# This script will:
#   - Load credentials from credentials.json
#   - Create admin user (if needed)
#   - Create test accounts (Claude Console, Gemini, etc.)
#   - Create test API keys
#   - Output test credentials for E2E scripts
#
###############################################################################

set -e  # Exit on error
set -u  # Exit on undefined variable
set -o pipefail  # Exit on pipeline failure

# ============================================================================
# Configuration
# ============================================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
CREDENTIALS_FILE="${SCRIPT_DIR}/credentials.json"
OUTPUT_FILE="${SCRIPT_DIR}/../.test-credentials"
BASE_URL="${BASE_URL:-http://localhost:8080}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# ============================================================================
# Utility Functions
# ============================================================================

log_info() {
    echo -e "${BLUE}[INFO]${NC} $1"
}

log_success() {
    echo -e "${GREEN}[✓]${NC} $1"
}

log_warning() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[✗]${NC} $1"
}

# ============================================================================
# Prerequisite Validation
# ============================================================================

log_info "Starting credentials injection..."
echo ""

# Check if credentials.json exists
if [ ! -f "${CREDENTIALS_FILE}" ]; then
    log_error "credentials.json not found!"
    log_error "Please copy credentials.json.example to credentials.json and fill in real data"
    echo ""
    echo "Quick start:"
    echo "  cp ${SCRIPT_DIR}/credentials.json.example ${CREDENTIALS_FILE}"
    echo "  vim ${CREDENTIALS_FILE}  # Edit with your real credentials"
    echo "  bash $0"
    exit 1
fi
log_success "credentials.json found"

# Check if jq is available
if ! command -v jq &> /dev/null; then
    log_error "jq is required but not installed"
    log_error "Install with: sudo apt-get install jq"
    exit 1
fi
log_success "jq is available"

# Check if backend is running
if ! curl -s -f "${BASE_URL}/health" > /dev/null 2>&1; then
    log_error "Backend not responding at ${BASE_URL}"
    log_error "Please start backend with: make rust-dev"
    exit 1
fi
log_success "Backend is healthy"

echo ""

# ============================================================================
# Load Credentials
# ============================================================================

log_info "Loading credentials from ${CREDENTIALS_FILE}..."

# Load admin credentials
ADMIN_USERNAME=$(jq -r '.admin.username' "${CREDENTIALS_FILE}")
ADMIN_PASSWORD=$(jq -r '.admin.password' "${CREDENTIALS_FILE}")

if [ -z "${ADMIN_USERNAME}" ] || [ "${ADMIN_USERNAME}" == "null" ]; then
    log_error "admin.username not found in credentials.json"
    exit 1
fi

if [ -z "${ADMIN_PASSWORD}" ] || [ "${ADMIN_PASSWORD}" == "null" ]; then
    log_error "admin.password not found in credentials.json"
    exit 1
fi

log_success "Admin credentials loaded"

# ============================================================================
# Admin Login
# ============================================================================

log_info "Performing admin login..."

LOGIN_RESPONSE=$(curl -s -X POST "${BASE_URL}/admin/auth/login" \
    -H "Content-Type: application/json" \
    -d "{\"username\":\"${ADMIN_USERNAME}\",\"password\":\"${ADMIN_PASSWORD}\"}")

# Extract token
ADMIN_TOKEN=$(echo "${LOGIN_RESPONSE}" | jq -r '.token')

if [ -z "${ADMIN_TOKEN}" ] || [ "${ADMIN_TOKEN}" == "null" ]; then
    log_error "Admin login failed"
    log_error "Response: ${LOGIN_RESPONSE}"
    exit 1
fi

log_success "Admin login successful"
echo ""

# ============================================================================
# Inject Claude Console Accounts
# ============================================================================

log_info "Injecting Claude Console accounts..."

CLAUDE_ACCOUNTS=$(jq -c '.accounts.claude_console[]?' "${CREDENTIALS_FILE}")
CLAUDE_ACCOUNT_IDS=()

if [ -z "${CLAUDE_ACCOUNTS}" ]; then
    log_warning "No Claude Console accounts in credentials.json"
else
    echo "${CLAUDE_ACCOUNTS}" | while IFS= read -r account; do
        ACCOUNT_NAME=$(echo "$account" | jq -r '.name')
        SESSION_TOKEN=$(echo "$account" | jq -r '.session_token')
        CUSTOM_ENDPOINT=$(echo "$account" | jq -r '.custom_api_endpoint // "https://api.claude.ai"')
        PRIORITY=$(echo "$account" | jq -r '.priority // 50')
        ACTIVE=$(echo "$account" | jq -r '.active // true')

        log_info "Creating account: ${ACCOUNT_NAME}"

        # Create account via admin API
        CREATE_RESPONSE=$(curl -s -X POST "${BASE_URL}/admin/claude-console-accounts" \
            -H "Authorization: Bearer ${ADMIN_TOKEN}" \
            -H "Content-Type: application/json" \
            -d "{
                \"name\": \"${ACCOUNT_NAME}\",
                \"description\": \"E2E Test Account (from credentials.json)\",
                \"session_token\": \"${SESSION_TOKEN}\",
                \"custom_api_endpoint\": \"${CUSTOM_ENDPOINT}\",
                \"priority\": ${PRIORITY},
                \"isActive\": ${ACTIVE}
            }")

        ACCOUNT_ID=$(echo "${CREATE_RESPONSE}" | jq -r '.data.id')

        if [ -z "${ACCOUNT_ID}" ] || [ "${ACCOUNT_ID}" == "null" ]; then
            log_error "Failed to create account: ${ACCOUNT_NAME}"
            log_error "Response: ${CREATE_RESPONSE}"
        else
            log_success "Created account: ${ACCOUNT_NAME} (ID: ${ACCOUNT_ID})"
            CLAUDE_ACCOUNT_IDS+=("${ACCOUNT_ID}")
        fi
    done
fi

echo ""

# ============================================================================
# Create Test API Key
# ============================================================================

log_info "Creating test API key..."

# Get all account IDs from Redis (fallback if injection failed)
if [ ${#CLAUDE_ACCOUNT_IDS[@]} -eq 0 ]; then
    log_warning "No accounts created, creating API key without account binding"
    ACCOUNT_BINDING=""
else
    # Convert array to JSON array string
    ACCOUNT_BINDING=$(printf '%s\n' "${CLAUDE_ACCOUNT_IDS[@]}" | jq -R . | jq -s -c .)
fi

API_KEY_RESPONSE=$(curl -s -X POST "${BASE_URL}/admin/api-keys" \
    -H "Authorization: Bearer ${ADMIN_TOKEN}" \
    -H "Content-Type: application/json" \
    -d "{
        \"name\": \"E2E-Test-Key-$(date +%Y%m%d-%H%M%S)\",
        \"permissions\": \"all\",
        \"claudeAccountIds\": ${ACCOUNT_BINDING:-[]}
    }")

TEST_API_KEY=$(echo "${API_KEY_RESPONSE}" | jq -r '.data.key')
TEST_API_KEY_ID=$(echo "${API_KEY_RESPONSE}" | jq -r '.data.id')

if [ -z "${TEST_API_KEY}" ] || [ "${TEST_API_KEY}" == "null" ]; then
    log_error "Failed to create API key"
    log_error "Response: ${API_KEY_RESPONSE}"
    exit 1
fi

log_success "Created API key: ${TEST_API_KEY:0:30}..."
echo ""

# ============================================================================
# Output Test Credentials
# ============================================================================

log_info "Writing test credentials to ${OUTPUT_FILE}..."

cat > "${OUTPUT_FILE}" <<EOF
# Automated Test Credentials
# Generated: $(date -u +"%Y-%m-%d %H:%M:%S UTC")
# Source: ${CREDENTIALS_FILE}
# DO NOT COMMIT THIS FILE

# Admin credentials
export ADMIN_TOKEN="${ADMIN_TOKEN}"
export ADMIN_USERNAME="${ADMIN_USERNAME}"

# Test API key
export TEST_API_KEY="${TEST_API_KEY}"
export TEST_API_KEY_ID="${TEST_API_KEY_ID}"

# Backend configuration
export BASE_URL="${BASE_URL}"

# Account IDs (for verification)
export CLAUDE_ACCOUNT_IDS=(${CLAUDE_ACCOUNT_IDS[@]})
EOF

chmod 600 "${OUTPUT_FILE}"
log_success "Credentials written to ${OUTPUT_FILE}"

echo ""
echo "╔════════════════════════════════════════════════════════════╗"
echo "║              Credentials Injection Complete                 ║"
echo "╚════════════════════════════════════════════════════════════╝"
echo ""
echo "✅ Admin Token:     ${ADMIN_TOKEN:0:20}..."
echo "✅ Test API Key:    ${TEST_API_KEY:0:30}..."
echo "✅ Test API Key ID: ${TEST_API_KEY_ID}"
echo "✅ Accounts:        ${#CLAUDE_ACCOUNT_IDS[@]} Claude Console account(s)"
echo ""
echo "Credentials file:   ${OUTPUT_FILE}"
echo ""
echo "🚀 Ready for E2E testing!"
echo ""
echo "To run E2E tests:"
echo "  source ${OUTPUT_FILE}"
echo "  bash tests/e2e/test-claudeconsole-e2e.sh 60"
echo ""
