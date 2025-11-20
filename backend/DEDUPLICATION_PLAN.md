# Main.rs Deduplication Plan

## Current State
- **main.rs**: 6,019 lines, 96 functions
- Contains: AppState with raw sled DB, Wordle game (163 lines), IoT devices (215 lines), 58 CRUD routes, backup logic

## What We Already Have in Separate Modules
- ✅ `db.rs` (153 lines) - Clean Database wrapper with insert/get/list/delete/update
- ✅ `backup.rs` (293 lines) - BackupManager with periodic backups
- ✅ `config.rs` (524 lines) - Configuration loading
- ✅ `logging.rs` (190 lines) - Logging setup
- ✅ `handlers/auth.rs` (695 lines) - Authentication with cookies
- ✅ `validation.rs` (111 lines) - Input validation

## Deduplication Strategy

### 1. Remove from main.rs
- ❌ **Wordle game code** (163 lines) - Not core to auth/business logic
- ❌ **IoT device clients** (215 lines) - kitchen_iot, cashier, display, pos, inventory
- ❌ **Custom AppState backup logic** - Use BackupManager instead
- ❌ **Raw sled access** - Use Database wrapper
- ❌ **Duplicate serve_wasm** - Already in routes/static_files.rs
- ❌ **Custom config CRUD routes** (if not essential) - custom_names, custom_zones, etc.

### 2. Keep Essential Routes
- ✅ Health check (already extracted to routes/health.rs)
- ✅ Auth endpoints (register, login, logout, refresh, me)
- ✅ Business entities: customers, quotes, invoices, certificates
- ✅ Static file serving + SPA fallback

### 3. Simplify AppState
**Before** (in main.rs):
```rust
struct AppState {
    db: Arc<RwLock<sled::Db>>,  // Raw sled
    backup_path: String,
    paseto_local_key: Arc<RwLock<[u8; 32]>>,
    // ... IoT clients
    // ... Wordle game state
}
```

**After**:
```rust
struct AppState {
    db: Database,  // Use wrapper from db.rs
    cfg: AppConfig,
}
```

### 4. Target Size
- **Goal**: Reduce main.rs from 6,000 lines to ~500-800 lines
- **Focus**: Server setup, route registration, minimal state

## Implementation Steps
1. Create new streamlined main.rs
2. Remove Wordle, IoT, custom config routes
3. Use Database wrapper instead of raw sled
4. Test compilation
5. Test basic auth flow

## Lines to Save
- Remove Wordle: ~163 lines
- Remove IoT: ~215 lines
- Remove custom CRUD: ~500+ lines
- Remove duplicate backup: ~200 lines
- Remove duplicate types: ~100 lines
- **Total savings**: ~1,200+ lines (20% reduction minimum)
