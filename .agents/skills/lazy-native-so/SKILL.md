---
name: lazy-native-so
description: Create or maintain a lazy-built native .so library that is hash-checked and dlopen'd at runtime, following the spike / nzea pattern. Use when adding a new simulator backend or external C/C++ dependency that should not block cargo build.
---

# Lazy Native `.so` Build Pattern

Use this skill when a crate needs to link against a heavy native library (C/C++/Verilator) whose build time should be deferred to the first call at runtime rather than running during `cargo build`.

## Architecture

```
crate/
├── build.rs              ← empty (or finds compile-time paths like zlib)
├── src/
│   ├── runtime.rs        ← hash-based freshness + build + dlopen2 load
│   ├── ffi.rs            ← API table (dlopen2 `WrapperApi` struct)
│   └── lib.rs            ← mod_prv!(ffi, runtime, ...)
```

## Runtime Module (`src/runtime.rs`)

### 1. `.so` output directory

Use `find_workspace_root()` to locate the workspace root, then nest under `target/remu-so/<crate>/`:

```rust
fn so_dir() -> PathBuf {
    find_workspace_root().join("target").join("remu-so").join("mycrate")
}
```

### 2. Source hash for incremental builds

Recursively hash all source files that affect the `.so` with SHA-256.
Store the hash in a `.stamp` file next to the `.so`.
On subsequent calls, compare the stored hash with the current source hash — skip rebuild if they match.

### 3. Build function

Steps follow the build pipeline (configure → make → compile wrapper → link).
Use `StepSpinner` (wraps `nanospinner`) to show progress with step counter:

```rust
let mut spinner = StepSpinner::new(total_steps, "message");
// after each step:
spinner.inc("next step message");
// on success:
spinner.done();
// on failure:
spinner.fail("reason");
```

All build commands should use `run_silent()` (success = quiet, failure = print last 20 lines of stderr).

### 4. Loading with dlopen2

Cache the loaded library in a static. `Container` keeps the library handle and
the symbol table together, so symbols can never outlive the library:

```rust
static LIBS: OnceLock<Result<Container<MyApi>, String>> = OnceLock::new();

pub(crate) fn ensure_loaded() -> Result<&'static Container<MyApi>, String> {
    LIBS.get_or_init(|| {
        if !fresh { build_so()?; }
        let cont = unsafe { Container::load(so_path()) }
            .map_err(|e| format!("dlopen {}: {e}", so_path().display()))?;
        Ok(cont)
    }).as_ref().map_err(|e| e.clone())
}
```

Hand the container out to callers (`&'static Container<MyApi>`); method calls go
through its `Deref` to the API struct (`api.step(ctx)`). If entries live in a
map, box them so a rehash cannot move them (nzea does).

## FFI Module (`src/ffi.rs`)

Describe the C ABI as a `#[derive(WrapperApi)]` (dlopen2) struct. The derive
requires fields to be private and fn-pointer parameters to be named; the field
name is the symbol name unless `#[dlopen2_name = "..."]` maps it to an exported
symbol. `Option<fn>` fields are optional symbols (`None` when absent):

```rust
#[derive(WrapperApi)]
pub(crate) struct MyApi {
    #[dlopen2_name = "spike_difftest_init"]
    init: unsafe extern "C" fn(ctx: *mut c_void) -> *mut c_void,
    step: unsafe extern "C" fn(ctx: *mut c_void) -> c_int,
    optional_probe: Option<unsafe extern "C" fn() -> c_int>,
}
```

The derive generates methods named after the fields (`api.init(ctx)`), mirroring
the fn pointer's unsafety; loading happens once in `Container::load` and fails
fast on a missing required symbol.

## Reference implementations

- `remu_simulator/simulators/spike/src/runtime.rs` — single target
- `remu_simulator/simulators/nzea/src/runtime.rs` — per-(platform, isa) combination
