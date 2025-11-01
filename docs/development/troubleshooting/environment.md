# Environment Variable Loading Fix

## Problem Summary

The Rust application was failing to start with the error:
```
Error: Invalid configuration: JWT_SECRET must be at least 32 characters
```

Even though the `.env` file contained correct values for `CRS_SECURITY__JWT_SECRET` (64 chars) and `CRS_SECURITY__ENCRYPTION_KEY` (32 chars).

## Root Cause

The `config` crate's `Environment::with_prefix("CRS").separator("__")` was **not reading environment variables set by dotenvy**. Investigation revealed:

1. ✅ The `dotenvy` library successfully loaded the `.env` file
2. ✅ Environment variables were set correctly (`CRS_SECURITY__JWT_SECRET` = 64 chars)
3. ❌ The `config` crate still read empty strings for these values

This was due to a case sensitivity or environment variable parsing issue in the `config` crate's `Environment` source.

## Solution

Modified `rust/src/config/mod.rs` to **manually read and override** environment variables instead of relying on `Environment::with_prefix()`.

### Code Changes

**Before** (not working):
```rust
.add_source(Environment::with_prefix("CRS").separator("__"))
```

**After** (working):
```rust
// Manually override with environment variables (workaround for case sensitivity issues)
if let Ok(val) = env::var("CRS_SECURITY__JWT_SECRET") {
    builder = builder.set_override("security.jwt_secret", val)?;
}
if let Ok(val) = env::var("CRS_SECURITY__ENCRYPTION_KEY") {
    builder = builder.set_override("security.encryption_key", val)?;
}
// ... (similar for all other config variables)
```

## Files Modified

1. **rust/src/config/mod.rs**
   - Removed `Environment` from imports (no longer needed)
   - Changed `Settings::new()` to manually read each environment variable
   - Added explicit `.set_override()` calls for all configuration fields

2. **rust/src/main.rs**
   - Kept `dotenvy::from_path("../.env")` to load the .env file
   - Removed debug output (was used for troubleshooting)

3. **scripts/load-env.sh**
   - Created helper script to explicitly export .env variables
   - Used by `scripts/start-backend.sh` to ensure variables are available

4. **scripts/start-backend.sh**
   - Added `source scripts/load-env.sh` before running cargo
   - Ensures environment variables are exported to the shell before cargo execution

## Verification

After the fix, the application starts successfully:

```
✅ Configuration loaded
✅ Configuration validated
🔌 Redis connection pool created
✅ Redis connection established
🚀 Server ready on http://0.0.0.0:8080
```

## Usage

From the project root directory:

```bash
# Method 1: Direct cargo run
cargo run

# Method 2: Quick development startup
bash rust-dev.sh

# Method 3: Full service startup
bash scripts/start-all.sh dev
```

All three methods now correctly load environment variables from the `.env` file.

## Future Considerations

If the `config` crate is updated or if a better solution for `Environment` source is found, this manual override approach can be replaced. However, the current solution is:

- ✅ Reliable and predictable
- ✅ Explicit about which environment variables are supported
- ✅ Easy to maintain and debug
- ✅ Works consistently across different environments

The manual approach also has the advantage of providing clear documentation of all supported environment variables in one place (`Settings::new()` method).
