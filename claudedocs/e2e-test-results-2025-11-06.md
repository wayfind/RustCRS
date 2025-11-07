# E2E Test Results - 2025-11-06

## Test Setup

### Environment
- **Backend**: Rust service (debug build)
- **Redis**: Docker container `redis-dev` (port 6379)
- **Test Data**: Injected via `tests/e2e/inject-test-data.sh`

### Test Credentials
- **API Key**: `cr_test_e2e_regression_001`
- **Account UUID**: `ccf9fee5-f6fc-40af-95c5-52bba152e89e`
- **Account Name**: E2E-Test-Claude-Console-001
- **Claude Console Endpoint**: https://us3.pincc.ai/api
- **Session Token**: cr_022dc9fc... (truncated)

## Test Results

### ✅ Successful Components

1. **Data Injection Script**
   - Redis FLUSHDB successful
   - Claude Console account injected correctly
   - API Key injected with SHA-256 hashing
   - Admin credentials injected from init.json
   - All data clearly visible in Redis

2. **Backend Service**
   - Service starts successfully
   - Health endpoint responds correctly
   - All services initialized (account, API key, scheduler, relay)
   - Static file serving configured

3. **Redis Data Verification**
   - Account data stored with complete structure
   - API Key hash mapping: `api_key_hash:{sha256}` → UUID
   - Account set: `claude:accounts` contains UUID
   - All fields properly formatted

### ❌ Bug Discovered: Field Name Mismatch

**Issue ID**: ISSUE-E2E-001
**Priority**: P0 (Blocks relay functionality)

**Description**:
The relay service fails with deserialization error when processing API requests.

**Error Message**:
```json
{
  "error": {
    "message": "反序列化失败: missing field `key_hash` at line 16 column 1",
    "status": 500,
    "type": "internal_error"
  }
}
```

**Root Cause**:
Schema mismatch between Redis data and Rust struct deserialization:
- **Injection script** (`tests/e2e/inject-test-data.sh:305`): Stores field as `"hashedKey"`
- **Rust code expects**: Field name `key_hash`

**Affected Files**:
- `tests/e2e/inject-test-data.sh` (line 305)
- Rust API Key struct (likely `rust/src/models/api_key.rs`)

**Redis Data Structure (Current)**:
```json
{
  "id": "08464766-9fec-4b23-a2db-0487d1eafed8",
  "name": "E2E-Test-Key-001",
  "hashedKey": "2bfdca858ddfc0f6...",  ← PROBLEM: Should be "key_hash"
  "permissions": "all",
  "claudeConsoleAccountIds": ["ccf9fee5-f6fc-40af-95c5-52bba152e89e"],
  ...
}
```

**Impact**:
- All API relay requests fail with 500 error
- Cannot test relay functionality
- Blocks all E2E testing scenarios

**Solution**:
Need to fix field name consistency. Options:
1. **Option A**: Change injection script to use `key_hash` (align with Rust code)
2. **Option B**: Change Rust struct to use `hashedKey` (align with JavaScript convention)

**Recommendation**: Option A (use `key_hash` in injection script) because:
- Less code to change (one script vs potentially multiple Rust files)
- Maintains Rust naming conventions (snake_case)
- No risk of breaking existing production data

## Test Request Details

**Request**:
```bash
curl -X POST http://localhost:8080/api/v1/messages \
  -H "Content-Type: application/json" \
  -H "Authorization: Bearer cr_test_e2e_regression_001" \
  -H "anthropic-version: 2023-06-01" \
  -d '{
    "model": "claude-3-5-sonnet-20241022",
    "max_tokens": 100,
    "messages": [
      {"role": "user", "content": "Say hello in one sentence"}
    ]
  }'
```

**Response**: 500 Internal Server Error (field name mismatch)

## Data Clarity Verification

### Redis Keys Verified
- ✅ `claude:accounts` → Set containing account UUIDs
- ✅ `claude_account:{uuid}` → Complete account JSON
- ✅ `api_key:{uuid}` → Complete API key JSON
- ✅ `api_key_hash:{sha256}` → UUID mapping for fast lookup

### Data Structure Clarity
All data in Redis is **clearly readable and well-structured**:
- JSON format with proper field names
- Timestamps in ISO 8601 format
- UUIDs properly generated
- SHA-256 hashes correctly computed

## Discovered Issues Summary

### ISSUE-E2E-001: Field Name Mismatch - hashedKey vs key_hash
**Status**: ✅ FIXED
**Location**: `tests/e2e/inject-test-data.sh` line 305
**Fix**: Changed `"hashedKey"` to `"key_hash"`

### ISSUE-E2E-002: Systemic Schema Mismatch (P0 - Critical)
**Status**: ❌ IN PROGRESS
**Priority**: P0 (Blocks all relay functionality)

**Root Cause**:
E2E injection script uses JavaScript camelCase conventions, but Rust backend expects snake_case for struct field deserialization.

**Affected Fields**:
- `isActive` → `is_active`
- `isDeleted` → `is_deleted`
- `tokenLimit` → `token_limit`
- `concurrencyLimit` → `concurrency_limit`
- `dailyCostLimit` → `daily_cost_limit`
- `totalCostLimit` → `total_cost_limit`
- `weeklyOpusCostLimit` → `weekly_opus_cost_limit`
- `enableModelRestriction` → `enable_model_restriction`
- `restrictedModels` → `restricted_models`
- `enableClientRestriction` → `enable_client_restriction`
- `allowedClients` → `allowed_clients`
- `expirationMode` → `expiration_mode`
- `activationDays` → `activation_days`
- `activationUnit` → `activation_unit`
- `createdAt` → `created_at`
- `updatedAt` → `updated_at`
- `lastUsedAt` → `last_used_at`

**Impact**:
- All API relay requests fail with deserialization errors
- Cannot test any relay functionality
- Blocks all E2E testing scenarios

**Solution**:
Rewrite `tests/e2e/inject-test-data.sh` API key JSON generation (lines 301-317) to use snake_case for ALL field names to match Rust struct definitions in `rust/src/models/api_key.rs`.

## Schema Fix Progress

### ISSUE-E2E-002: API Key Schema - COMPLETED ✅

**Final Solution**: Mixed naming convention matching Rust serde configuration
- Fields with `#[serde(rename = "...")]` use camelCase: `isActive`, `isDeleted`, `createdAt`, `updatedAt`, `lastUsedAt`
- Fields without rename use snake_case: `key_hash`, `token_limit`, `concurrency_limit`, etc.

**Verification**: API Key deserialization successful, no more 500 errors

### ISSUE-E2E-003: Account List Key Mismatch - FIXED ✅

**Status**: ✅ FIXED
**Location**: `tests/e2e/inject-test-data.sh` line 230
**Fix**: Changed `claude:accounts` to `claude_accounts`

**Root Cause**: Redis SET key name mismatch
- Injection script used: `claude:accounts` (with colon)
- Rust code expected: `claude_accounts` (without colon)

**Evidence**:
- Account service found accounts after fix
- Scheduler selected account successfully

### ISSUE-E2E-004: Account ID Prefix Mismatch - FIXED ✅

**Status**: ✅ FIXED
**Priority**: P0 (Was blocking relay functionality)
**Location**: `rust/src/routes/api.rs` lines 189, 302, 310, 345

**Description**: Relay service could not find account because of ID prefix mismatch

**Root Cause**: Account ID format inconsistency
- Redis storage key: `claude_account:{uuid}`  (e.g., `claude_account:3810df51-4b93-4403-9f27-083528dd8d74`)
- API routes were adding prefix: `claude_acc_{uuid}` (e.g., `claude_acc_3810df51-4b93-4403-9f27-083528dd8d74`)
- Relay service tried to load with wrong prefix ← KEY NOT FOUND

**Fix Applied**:
Changed all 4 instances from:
```rust
format!("claude_acc_{}", selected.account.id)
```
To:
```rust
selected.account.id.to_string()
```

**Verification**:
- Before fix: "Account not found" (404)
- After fix: Account found successfully, request proceeds to client validation

**Impact Resolved**:
- ✅ Account lookup now works correctly
- ✅ Relay functionality unblocked
- ✅ Can proceed with E2E testing

## Next Steps

1. **Fix Account Schema** (ISSUE-E2E-003)
   - Check `rust/src/models/claude_account.rs` for field names and serde configuration
   - Update `tests/e2e/inject-test-data.sh` account JSON structure
   - Re-inject test data
   - Verify account discovery works

2. **Continue E2E Testing**
   - Test successful relay request
   - Verify data flow in Redis after relay
   - Check usage tracking
   - Verify account scheduling

3. **Document Additional Issues**
   - Add to issue-todo.md as discovered
   - Priority based on impact

## Conclusion

The E2E testing infrastructure continues to **validate effectively** - discovering critical schema mismatches between test data injection and Rust struct deserialization. The API Key schema issue (ISSUE-E2E-002) has been resolved, demonstrating the iterative debugging approach works well.

**Testing Status**: 🟡 API Key schema fixed, account discovery issue identified, continuing systematic fixes.
