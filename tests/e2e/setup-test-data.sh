#!/bin/bash
###############################################################################
# Automated Test Data Setup Script
#
# Purpose: Create/verify test data as prerequisite for all regression tests
# Usage: bash tests/regression/setup-test-data.sh [SESSION_TOKEN]
#
# This script is the foundation for all regression tests, providing:
# - Automated admin authentication from init.json
# - Idempotent test account creation
# - Idempotent test API key creation
# - Credential output for test script consumption
#
# Requirements:
# - Backend running on localhost:8080
# - Redis accessible
# - rust/data/init.json exists with admin credentials
# - (Optional) Claude Console session token for account creation
###############################################################################

set -e  # Exit on error
set -u  # Exit on undefined variable
set -o pipefail  # Exit on pipeline failure

# ============================================================================
# Configuration
# ============================================================================

BASE_URL="http://localhost:8080"
INIT_JSON="rust/data/init.json"
OUTPUT_FILE="tests/.test-credentials"
TEST_ACCOUNT_NAME="E2E-Test-Account"
TEST_API_KEY_NAME="E2E-Test-Key"

# Session token can be provided via:
# 1. Command line argument: $1
# 2. Environment variable: CLAUDE_SESSION_TOKEN
# 3. Interactive prompt (if neither provided)
SESSION_TOKEN="${1:-${CLAUDE_SESSION_TOKEN:-}}"

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

verify_prerequisites() {
    log_info "Verifying prerequisites..."

    # Check if backend is running
    if ! curl -s -f "${BASE_URL}/health" > /dev/null 2>&1; then
        log_error "Backend not responding at ${BASE_URL}"
        log_error "Please start backend with: make rust-dev"
        exit 1
    fi
    log_success "Backend is healthy"

    # Check if init.json exists
    if [ ! -f "${INIT_JSON}" ]; then
        log_error "Init file not found: ${INIT_JSON}"
        exit 1
    fi
    log_success "Init file found"

    # Check if jq is available
    if ! command -v jq &> /dev/null; then
        log_error "jq is required but not installed"
        log_error "Install with: sudo apt-get install jq"
        exit 1
    fi
    log_success "jq is available"
}

# ============================================================================
# Core Functions
# ============================================================================

# Load admin credentials from init.json
load_admin_credentials() {
    log_info "Loading admin credentials from ${INIT_JSON}..."

    ADMIN_USERNAME=$(jq -r '.adminUsername' "${INIT_JSON}")
    ADMIN_PASSWORD=$(jq -r '.adminPassword' "${INIT_JSON}")

    if [ -z "${ADMIN_USERNAME}" ] || [ "${ADMIN_USERNAME}" == "null" ]; then
        log_error "Failed to load adminUsername from ${INIT_JSON}"
        exit 1
    fi

    if [ -z "${ADMIN_PASSWORD}" ] || [ "${ADMIN_PASSWORD}" == "null" ]; then
        log_error "Failed to load adminPassword from ${INIT_JSON}"
        exit 1
    fi

    log_success "Loaded credentials for user: ${ADMIN_USERNAME}"
}

# Perform admin login and get JWT token
admin_login() {
    log_info "Performing admin login..."

    local response
    response=$(curl -s -X POST "${BASE_URL}/admin/auth/login" \
        -H "Content-Type: application/json" \
        -d "{\"username\":\"${ADMIN_USERNAME}\",\"password\":\"${ADMIN_PASSWORD}\"}")

    # Check if response is valid JSON
    if ! echo "${response}" | jq . > /dev/null 2>&1; then
        log_error "Login failed: Invalid JSON response"
        log_error "Response: ${response}"
        exit 1
    fi

    # Extract token
    ADMIN_TOKEN=$(echo "${response}" | jq -r '.token')

    if [ -z "${ADMIN_TOKEN}" ] || [ "${ADMIN_TOKEN}" == "null" ]; then
        log_error "Login failed: No token in response"
        log_error "Response: ${response}"
        exit 1
    fi

    log_success "Admin login successful"
}

# Ensure test Claude Console account exists
ensure_test_account() {
    log_info "Checking for existing test account: ${TEST_ACCOUNT_NAME}..."

    # Get all Claude Console accounts
    local accounts
    accounts=$(curl -s -X GET "${BASE_URL}/admin/claude-accounts" \
        -H "Authorization: Bearer ${ADMIN_TOKEN}")

    # Search for account by name
    TEST_ACCOUNT_ID=$(echo "${accounts}" | jq -r \
        ".data[]? | select(.name == \"${TEST_ACCOUNT_NAME}\") | .id" 2>/dev/null || echo "")

    if [ -n "${TEST_ACCOUNT_ID}" ] && [ "${TEST_ACCOUNT_ID}" != "null" ]; then
        log_success "Test account already exists: ${TEST_ACCOUNT_ID}"
        return 0
    fi

    # Account doesn't exist, create it
    log_info "Creating new test account..."

    # Prompt for session token if not provided
    if [ -z "${SESSION_TOKEN}" ]; then
        log_warning "Session token not provided"
        echo -n "Enter Claude Console session token (or press Enter to skip account creation): "
        read -r SESSION_TOKEN

        if [ -z "${SESSION_TOKEN}" ]; then
            log_warning "Skipping test account creation (no session token)"
            TEST_ACCOUNT_ID=""
            return 0
        fi
    fi

    # Create account
    local create_response
    create_response=$(curl -s -X POST "${BASE_URL}/admin/claude-accounts" \
        -H "Authorization: Bearer ${ADMIN_TOKEN}" \
        -H "Content-Type: application/json" \
        -d "{
            \"name\": \"${TEST_ACCOUNT_NAME}\",
            \"platform\": \"ClaudeConsole\",
            \"session_token\": \"${SESSION_TOKEN}\",
            \"custom_api_endpoint\": \"https://us3.pincc.ai/api\",
            \"active\": true
        }")

    TEST_ACCOUNT_ID=$(echo "${create_response}" | jq -r '.data.id')

    if [ -z "${TEST_ACCOUNT_ID}" ] || [ "${TEST_ACCOUNT_ID}" == "null" ]; then
        log_error "Failed to create test account"
        log_error "Response: ${create_response}"
        exit 1
    fi

    log_success "Test account created: ${TEST_ACCOUNT_ID}"
}

# Ensure test API key exists
ensure_test_api_key() {
    log_info "Checking for existing test API key: ${TEST_API_KEY_NAME}..."

    # Get all API keys
    local api_keys
    api_keys=$(curl -s -X GET "${BASE_URL}/admin/api-keys" \
        -H "Authorization: Bearer ${ADMIN_TOKEN}")

    # Search for API key by name
    local existing_key_id
    existing_key_id=$(echo "${api_keys}" | jq -r \
        ".data[]? | select(.name == \"${TEST_API_KEY_NAME}\") | .id" 2>/dev/null || echo "")

    if [ -n "${existing_key_id}" ] && [ "${existing_key_id}" != "null" ]; then
        log_warning "Test API key already exists (name: ${TEST_API_KEY_NAME})"
        log_warning "Note: Existing key value cannot be retrieved"
        log_warning "If you need the key value, delete the existing key and re-run this script"

        # We cannot retrieve the actual key value, only return the ID
        TEST_API_KEY="<existing-key-value-unavailable>"
        TEST_API_KEY_ID="${existing_key_id}"
        log_success "Using existing API key ID: ${existing_key_id}"
        return 0
    fi

    # API key doesn't exist, create it
    log_info "Creating new test API key..."

    # Determine account IDs for permissions
    local account_ids="[]"
    if [ -n "${TEST_ACCOUNT_ID}" ] && [ "${TEST_ACCOUNT_ID}" != "null" ]; then
        account_ids="[\"${TEST_ACCOUNT_ID}\"]"
    fi

    # Create API key
    local create_response
    create_response=$(curl -s -X POST "${BASE_URL}/admin/api-keys" \
        -H "Authorization: Bearer ${ADMIN_TOKEN}" \
        -H "Content-Type: application/json" \
        -d "{
            \"name\": \"${TEST_API_KEY_NAME}\",
            \"permissions\": \"claude\",
            \"claudeConsoleAccountIds\": ${account_ids}
        }")

    TEST_API_KEY=$(echo "${create_response}" | jq -r '.data.key')
    TEST_API_KEY_ID=$(echo "${create_response}" | jq -r '.data.id')

    if [ -z "${TEST_API_KEY}" ] || [ "${TEST_API_KEY}" == "null" ]; then
        log_error "Failed to create test API key"
        log_error "Response: ${create_response}"
        exit 1
    fi

    log_success "Test API key created: ${TEST_API_KEY}"
}

# Output test configuration to file
output_test_config() {
    log_info "Writing test credentials to ${OUTPUT_FILE}..."

    # Create .gitignore entry if it doesn't exist
    local gitignore_file="tests/.gitignore"
    if [ ! -f "${gitignore_file}" ]; then
        echo ".test-credentials" > "${gitignore_file}"
        log_info "Created ${gitignore_file}"
    elif ! grep -q "^\.test-credentials$" "${gitignore_file}"; then
        echo ".test-credentials" >> "${gitignore_file}"
        log_info "Added .test-credentials to ${gitignore_file}"
    fi

    # Write credentials file
    cat > "${OUTPUT_FILE}" <<EOF
# Automated Test Credentials
# Generated: $(date -u +"%Y-%m-%d %H:%M:%S UTC")
# DO NOT COMMIT THIS FILE

# Admin credentials
export ADMIN_TOKEN="${ADMIN_TOKEN}"
export ADMIN_USERNAME="${ADMIN_USERNAME}"

# Test account
export TEST_ACCOUNT_ID="${TEST_ACCOUNT_ID:-}"
export TEST_ACCOUNT_NAME="${TEST_ACCOUNT_NAME}"
export TEST_SESSION_TOKEN="${SESSION_TOKEN:-}"

# Test API key
export TEST_API_KEY="${TEST_API_KEY:-}"
export TEST_API_KEY_ID="${TEST_API_KEY_ID:-}"
export TEST_API_KEY_NAME="${TEST_API_KEY_NAME}"

# Backend configuration
export BASE_URL="${BASE_URL}"
EOF

    chmod 600 "${OUTPUT_FILE}"
    log_success "Credentials written to ${OUTPUT_FILE}"
}

# Display summary
display_summary() {
    echo ""
    echo "=========================================="
    echo "Test Data Setup Complete"
    echo "=========================================="
    echo ""
    echo "Admin Token:        ${ADMIN_TOKEN:0:20}..."
    echo "Test Account ID:    ${TEST_ACCOUNT_ID:-<not created>}"
    echo "Test API Key:       ${TEST_API_KEY:-<existing or not created>}"
    echo "Test API Key ID:    ${TEST_API_KEY_ID:-<not created>}"
    echo ""
    echo "Credentials file:   ${OUTPUT_FILE}"
    echo ""
    echo "To use in test scripts:"
    echo "  source ${OUTPUT_FILE}"
    echo ""
}

# ============================================================================
# Main Execution
# ============================================================================

main() {
    log_info "Starting automated test data setup..."
    echo ""

    # Run setup steps
    verify_prerequisites
    echo ""

    load_admin_credentials
    echo ""

    admin_login
    echo ""

    ensure_test_account
    echo ""

    ensure_test_api_key
    echo ""

    output_test_config
    echo ""

    display_summary

    log_success "Test data setup completed successfully!"
}

# Run main function
main "$@"
