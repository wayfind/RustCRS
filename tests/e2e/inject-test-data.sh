#!/bin/bash
###############################################################################
# Redis Test Data Injection Script for E2E Tests
#
# Purpose: Clean Redis and inject test base data for end-to-end (E2E) tests
# Usage: bash tests/e2e/inject-test-data.sh
#
# This script:
# - Cleans Redis database (FLUSHDB)
# - Reads test-base-data.json
# - Injects Claude Console accounts directly into Redis
# - Injects API Keys with proper SHA-256 hashing
# - Injects required admin data from init.json
#
# Requirements:
# - Redis running (Docker container 'redis-dev' or localhost:6379)
# - tests/e2e/test-base-data.json exists with real credentials
# - rust/data/init.json exists with admin credentials
# - jq, sha256sum utilities
###############################################################################

set -e  # Exit on error
set -u  # Exit on undefined variable
set -o pipefail  # Exit on pipeline failure

# ============================================================================
# Configuration
# ============================================================================

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
BASE_DATA_FILE="${SCRIPT_DIR}/test-base-data.json"
INIT_JSON="${PROJECT_ROOT}/rust/data/init.json"
REDIS_HOST="${REDIS_HOST:-localhost}"
REDIS_PORT="${REDIS_PORT:-6379}"
REDIS_CONTAINER="${REDIS_CONTAINER:-redis-dev}"

# Colors
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
# Redis CLI Wrapper
# ============================================================================

# Run redis-cli command, automatically choosing between docker exec or direct connection
redis_cli() {
    # Try docker exec first if container exists
    if docker ps -q -f name="${REDIS_CONTAINER}" > /dev/null 2>&1; then
        docker exec -i "${REDIS_CONTAINER}" redis-cli "$@"
    elif command -v redis-cli &> /dev/null; then
        # Fallback to host redis-cli if available
        redis-cli -h "${REDIS_HOST}" -p "${REDIS_PORT}" "$@"
    else
        log_error "Neither Docker container '${REDIS_CONTAINER}' nor redis-cli command is available"
        exit 1
    fi
}

# ============================================================================
# Prerequisite Validation
# ============================================================================

verify_prerequisites() {
    log_info "Verifying prerequisites..."

    # Check Redis (using docker or direct connection)
    if ! redis_cli ping > /dev/null 2>&1; then
        log_error "Redis not responding"
        log_error "Please start Redis with: docker run -d --name redis-dev -p 6379:6379 redis:7-alpine"
        exit 1
    fi
    log_success "Redis is responding"

    # Check jq
    if ! command -v jq &> /dev/null; then
        log_error "jq is required but not installed"
        log_error "Install with: sudo apt-get install jq"
        exit 1
    fi
    log_success "jq is available"

    # Check sha256sum
    if ! command -v sha256sum &> /dev/null; then
        log_error "sha256sum is required but not installed"
        exit 1
    fi
    log_success "sha256sum is available"

    # Check base data file
    if [ ! -f "${BASE_DATA_FILE}" ]; then
        log_error "Base data file not found: ${BASE_DATA_FILE}"
        log_error "Please create it from test-base-data.json.example"
        exit 1
    fi
    log_success "Base data file found"

    # Check init.json
    if [ ! -f "${INIT_JSON}" ]; then
        log_error "Init file not found: ${INIT_JSON}"
        exit 1
    fi
    log_success "Init file found"
}

# ============================================================================
# Redis Operations
# ============================================================================

clean_redis() {
    log_info "Cleaning Redis database..."

    redis_cli FLUSHDB > /dev/null

    log_success "Redis database cleaned"
}

# ============================================================================
# Account Injection
# ============================================================================

inject_claude_console_accounts() {
    log_info "Injecting Claude Console accounts..."

    local account_count
    account_count=$(jq '.accounts.claude_console | length' "${BASE_DATA_FILE}")

    if [ "${account_count}" == "0" ] || [ "${account_count}" == "null" ]; then
        log_warning "No Claude Console accounts to inject"
        return 0
    fi

    log_info "Found ${account_count} Claude Console account(s)"

    for i in $(seq 0 $((account_count - 1))); do
        local account
        account=$(jq ".accounts.claude_console[${i}]" "${BASE_DATA_FILE}")

        local account_id=$(echo "${account}" | jq -r '.id')
        local name=$(echo "${account}" | jq -r '.name')
        local platform=$(echo "${account}" | jq -r '.platform')
        local session_token=$(echo "${account}" | jq -r '.session_token')
        local custom_api_endpoint=$(echo "${account}" | jq -r '.custom_api_endpoint')
        local active=$(echo "${account}" | jq -r '.active')
        local priority=$(echo "${account}" | jq -r '.priority')
        local schedulable=$(echo "${account}" | jq -r '.schedulable')
        local concurrency_limit=$(echo "${account}" | jq -r '.concurrency_limit')
        local use_unified_user_agent=$(echo "${account}" | jq -r '.use_unified_user_agent')
        local use_unified_client_id=$(echo "${account}" | jq -r '.use_unified_client_id')
        local auto_stop_on_warning=$(echo "${account}" | jq -r '.auto_stop_on_warning')

        # Generate UUID for the account
        local account_uuid
        if command -v uuidgen &> /dev/null; then
            account_uuid=$(uuidgen | tr '[:upper:]' '[:lower:]')
        else
            # Fallback: use simple UUID generation
            account_uuid=$(cat /proc/sys/kernel/random/uuid)
        fi

        local now=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

        # Build complete account JSON matching ClaudeAccount structure
        local account_json=$(cat <<EOF
{
  "id": "${account_uuid}",
  "name": "${name}",
  "description": null,
  "email": null,
  "password": null,
  "claudeAiOauth": null,
  "accessToken": null,
  "refreshToken": null,
  "session_token": "${session_token}",
  "custom_api_endpoint": "${custom_api_endpoint}",
  "expiresAt": null,
  "scopes": null,
  "proxy": null,
  "isActive": ${active},
  "accountType": "shared",
  "platform": "${platform}",
  "priority": ${priority},
  "schedulable": ${schedulable},
  "subscriptionInfo": null,
  "autoStopOnWarning": ${auto_stop_on_warning},
  "useUnifiedUserAgent": ${use_unified_user_agent},
  "useUnifiedClientId": ${use_unified_client_id},
  "unifiedClientId": null,
  "accountExpiresAt": null,
  "extInfo": null,
  "status": "active",
  "errorMessage": null,
  "lastRefreshAt": null,
  "concurrencyLimit": ${concurrency_limit},
  "currentConcurrency": 0,
  "createdAt": "${now}",
  "updatedAt": "${now}"
}
EOF
)

        # Store account in Redis
        local redis_key="claude_account:${account_uuid}"
        echo "${account_json}" | redis_cli -x SET "${redis_key}" > /dev/null

        # Add account ID to the account list (SET)
        redis_cli SADD "claude_accounts" "${account_uuid}" > /dev/null

        log_success "Injected account: ${name} (${account_uuid})"

        # Store account UUID for API key association
        echo "${account_uuid}" > "/tmp/test-account-${account_id}.uuid"
    done
}

# ============================================================================
# API Key Injection
# ============================================================================

inject_api_keys() {
    log_info "Injecting API keys..."

    local key_count
    key_count=$(jq '.api_keys.test_keys | length' "${BASE_DATA_FILE}")

    if [ "${key_count}" == "0" ] || [ "${key_count}" == "null" ]; then
        log_warning "No API keys to inject"
        return 0
    fi

    log_info "Found ${key_count} API key(s)"

    for i in $(seq 0 $((key_count - 1))); do
        local key_data
        key_data=$(jq ".api_keys.test_keys[${i}]" "${BASE_DATA_FILE}")

        local key_id=$(echo "${key_data}" | jq -r '.id')
        local name=$(echo "${key_data}" | jq -r '.name')
        local key=$(echo "${key_data}" | jq -r '.key')
        local permissions=$(echo "${key_data}" | jq -r '.permissions')
        local claude_console_account_ids=$(echo "${key_data}" | jq -r '.claude_console_account_ids')

        # Generate UUID for the key
        local key_uuid
        if command -v uuidgen &> /dev/null; then
            key_uuid=$(uuidgen | tr '[:upper:]' '[:lower:]')
        else
            key_uuid=$(cat /proc/sys/kernel/random/uuid)
        fi

        # Calculate SHA-256 hash of the key
        local key_hash
        key_hash=$(echo -n "${key}" | sha256sum | awk '{print $1}')

        # Resolve account UUIDs
        local account_uuids="[]"
        if [ -n "${claude_console_account_ids}" ] && [ "${claude_console_account_ids}" != "null" ]; then
            local uuid_array="["
            local first=true
            for account_id in $(echo "${claude_console_account_ids}" | jq -r '.[]'); do
                if [ -f "/tmp/test-account-${account_id}.uuid" ]; then
                    local account_uuid=$(cat "/tmp/test-account-${account_id}.uuid")
                    if [ "${first}" = true ]; then
                        uuid_array="${uuid_array}\"${account_uuid}\""
                        first=false
                    else
                        uuid_array="${uuid_array},\"${account_uuid}\""
                    fi
                fi
            done
            uuid_array="${uuid_array}]"
            account_uuids="${uuid_array}"
        fi

        local now=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

        # Build API key JSON
        local api_key_json=$(cat <<EOF
{
  "id": "${key_uuid}",
  "name": "${name}",
  "key_hash": "${key_hash}",
  "permissions": "${permissions}",
  "claudeConsoleAccountIds": ${account_uuids},
  "googleGeminiAccountIds": [],
  "openAiAccountIds": [],
  "isActive": true,
  "isDeleted": false,
  "token_limit": 0,
  "concurrency_limit": 0,
  "daily_cost_limit": 0.0,
  "total_cost_limit": 0.0,
  "weekly_opus_cost_limit": 0.0,
  "enable_model_restriction": false,
  "restricted_models": [],
  "enable_client_restriction": false,
  "allowed_clients": [],
  "tags": [],
  "expiration_mode": "fixed",
  "activation_days": 0,
  "activation_unit": "days",
  "createdAt": "${now}",
  "updatedAt": "${now}",
  "lastUsedAt": null
}
EOF
)

        # Store API key in Redis
        local api_key_redis_key="api_key:${key_uuid}"
        echo "${api_key_json}" | redis_cli -x SET "${api_key_redis_key}" > /dev/null

        # Store hash mapping for fast lookup
        local hash_key="api_key_hash:${key_hash}"
        redis_cli SET "${hash_key}" "${key_uuid}" > /dev/null

        log_success "Injected API key: ${name} (${key}) -> hash: ${key_hash:0:16}..."
    done
}

# ============================================================================
# Admin Data Injection
# ============================================================================

inject_admin_data() {
    log_info "Injecting admin data from init.json..."

    local admin_username
    admin_username=$(jq -r '.adminUsername' "${INIT_JSON}")

    local admin_password
    admin_password=$(jq -r '.adminPassword' "${INIT_JSON}")

    if [ -z "${admin_username}" ] || [ "${admin_username}" == "null" ]; then
        log_warning "No admin username found in init.json, skipping admin injection"
        return 0
    fi

    # Generate admin user UUID
    local admin_uuid
    if command -v uuidgen &> /dev/null; then
        admin_uuid=$(uuidgen | tr '[:upper:]' '[:lower:]')
    else
        admin_uuid=$(cat /proc/sys/kernel/random/uuid)
    fi

    local now=$(date -u +"%Y-%m-%dT%H:%M:%SZ")

    # Build admin user JSON (simplified structure)
    local admin_json=$(cat <<EOF
{
  "id": "${admin_uuid}",
  "username": "${admin_username}",
  "password": "${admin_password}",
  "role": "admin",
  "createdAt": "${now}",
  "updatedAt": "${now}"
}
EOF
)

    # Store admin user
    redis_cli SET "admin:${admin_uuid}" "${admin_json}" > /dev/null
    redis_cli SET "admin_username:${admin_username}" "${admin_uuid}" > /dev/null

    log_success "Injected admin user: ${admin_username}"
}

# ============================================================================
# Cleanup Temp Files
# ============================================================================

cleanup_temp_files() {
    rm -f /tmp/test-account-*.uuid
}

# ============================================================================
# Display Summary
# ============================================================================

display_summary() {
    echo ""
    echo "==========================================="
    echo "  Redis Test Data Injection Complete"
    echo "==========================================="
    echo ""
    echo "✓ Redis database cleaned"
    echo "✓ Claude Console accounts injected"
    echo "✓ API keys injected with proper hashing"
    echo "✓ Admin data injected from init.json"
    echo ""
    echo "Data source: ${BASE_DATA_FILE}"
    echo "Redis: ${REDIS_HOST}:${REDIS_PORT}"
    echo ""
}

# ============================================================================
# Main Execution
# ============================================================================

main() {
    log_info "Starting Redis test data injection..."
    echo ""

    verify_prerequisites
    echo ""

    clean_redis
    echo ""

    inject_claude_console_accounts
    echo ""

    inject_api_keys
    echo ""

    inject_admin_data
    echo ""

    cleanup_temp_files

    display_summary

    log_success "Test data injection completed successfully!"
}

# Run main function
main "$@"
