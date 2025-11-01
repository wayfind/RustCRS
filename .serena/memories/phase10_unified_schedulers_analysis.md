# Phase 10 - Unified Schedulers Analysis

## Overview

The unified schedulers are critical components that implement intelligent account selection for multi-platform AI API requests. There are three schedulers totaling ~2,949 lines of Node.js code:

1. **UnifiedClaudeScheduler** (1,444 lines) - Handles claude-official, claude-console, bedrock, ccr accounts
2. **UnifiedGeminiScheduler** (~750 lines est.) - Handles Gemini accounts  
3. **UnifiedOpenAIScheduler** (~755 lines est.) - Handles OpenAI-compatible accounts

## UnifiedClaudeScheduler - Detailed Analysis

### File: `/home/david/prj/claude-relay-service/src/services/unifiedClaudeScheduler.js`

#### Core Methods (20 total)

**Account Selection**:
- `selectAccountForApiKey(apiKeyData, sessionHash, requestedModel)` - Main entry point for account selection
- `selectAccountFromGroup(groupId, modelName, stickySessionHash)` - Select from specific account group
- `_getAllAvailableAccounts(apiKeyData, requestedModel, sessionHash)` - Get all eligible accounts

**Model Support Checking**:
- `_isModelSupportedByAccount(account, accountType, requestedModel, context)` - Comprehensive model support validation
  - Claude Official: Only claude-* models, Opus requires Max subscription
  - Claude Console: supportedModels array/object validation
  - CCR: supportedModels array/object validation
  - Bedrock: All claude models supported by default

**Account Availability**:
- `_isAccountAvailable(account, accountType, requestedModel)` - Check if account can handle request
  - Status must be 'active'
  - Schedulable flag check (兼容字符串和布尔值)
  - Rate limit status check
  - Concurrent request limits
  - Model support validation

**Sticky Session Management**:
- `_getSessionMapping(sessionHash)` - Retrieve session->account mapping
- `_setSessionMapping(sessionHash, accountId, accountType, ttlSeconds)` - Create session binding
- `_extendSessionMappingTTL(sessionHash, accountId, accountType, ttlSeconds)` - Renew session TTL
- `_deleteSessionMapping(sessionHash)` - Remove session binding

**Account Sorting**:
- `_sortAccountsByPriority(accounts)` - Sort by priority (lower number = higher priority)

**Rate Limiting & Error Handling**:
- `markAccountRateLimited(accountId, accountType, durationMinutes)` - Mark account as rate-limited (429)
- `removeAccountRateLimit(accountId, accountType)` - Clear rate limit flag
- `isAccountRateLimited(accountId, accountType)` - Check rate limit status
- `markAccountUnauthorized(accountId, accountType)` - Handle 401/403 errors
- `markAccountBlocked(accountId, accountType, durationMinutes)` - Handle blocking状态
- `blockConsoleAccount(accountId, durationMinutes)` - Specifically block Console accounts

**CCR Account Handling**:
- `_selectCcrAccount(ccrAccounts, requestedModel)` - CCR-specific account selection
- `_getAvailableCcrAccounts(requestedModel)` - Get all available CCR accounts

### Key Features

1. **Multi-Account-Type Support**: Handles 4 account types (claude-official, claude-console, bedrock, ccr)

2. **Vendor Prefix Parsing**: `ccr:claude-3-5-sonnet` → vendor=ccr, baseModel=claude-3-5-sonnet

3. **Sticky Sessions**: Same sessionHash always uses same account (with TTL and renewal)

4. **Priority-Based Selection**: Accounts sorted by priority field (lower = higher priority)

5. **Concurrent Request Management**: Each account has maxConcurrentRequests limit

6. **Rate Limit Handling**: 429 errors trigger temporary account exclusion

7. **Model Compatibility**: Sophisticated model支持检查 logic per account type

8. **Account Group Support**: Can select from specific account groups

### Redis Keys Used

- `unified_claude_session_mapping:{sessionHash}` - Session sticky mapping
- `rate_limit:account:{accountId}` - Rate limit flag
- `account_blocked:{accountId}` - Blocked account flag

### Selection Algorithm

```
1. Parse vendor prefix from model name (if any)
2. Check for sticky session mapping
   - If exists and account available → return cached account
   - If exists but account unavailable → continue to fresh selection
3. Get all available accounts (filter by type, status, schedulable, model support)
4. For each account type priority (Official > Console > Bedrock > CCR):
   a. Filter accounts by type
   b. Check model support
   c. Check rate limits
   d. Check concurrent request limits
   e. Sort by priority field
   f. Select first available
5. If CCR accounts available and model matches:
   - Apply CCR-specific selection logic
6. Create session mapping for selected account (if sessionHash provided)
7. Return {accountId, accountType, accountService}
```

### Error Scenarios

- **No Available Accounts**: Throw `No available accounts for requested model`
- **Rate Limited**: Temporarily exclude account (configurable duration)
- **Unauthorized**: Mark account status as unauthorized
- **Blocked**: Temporarily exclude account

### Configuration Points

- Sticky session TTL: Configured via environment variable
- Rate limit duration: Passed to markAccountRateLimited()
- Account priorities: Stored in account.priority field
- Max concurrent requests: Stored in account.maxConcurrentRequests

## Implementation Strategy for Rust

Given the complexity (nearly 3,000 lines total), the implementation should be phased:

**Phase 10.1**: Core UnifiedClaudeScheduler structure
- Account selection core logic
- Model support checking
- Basic sticky session support

**Phase 10.2**: Rate limiting and error handling
- markAccountRateLimited, removeAccountRateLimit
- markAccountUnauthorized, markAccountBlocked
- isAccountRateLimited

**Phase 10.3**: Advanced features
- Account groups
- CCR-specific logic
- Session TTL renewal
- Priority-based selection refinement

**Phase 10.4**: UnifiedGeminiScheduler

**Phase 10.5**: UnifiedOpenAIScheduler

**Phase 10.6**: Integration tests

### Dependencies Needed

- ClaudeAccountService (account.rs - already implemented)
- AccountScheduler (account_scheduler.rs - already implemented)
- RedisPool (already available)
- Model parsing utilities
- Concurrent request tracking (already in AccountScheduler)

## Next Steps

1. Create Rust traits for unified scheduler interface
2. Implement UnifiedClaudeScheduler core structure
3. Port account selection logic method by method
4. Add comprehensive integration tests
5. Repeat for Gemini and OpenAI schedulers

Total estimated lines for Rust implementation: ~2,000-2,500 lines (more concise than Node.js)
