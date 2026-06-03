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
│   ├── runtime.rs        ← hash-based freshness + build + libloading
│   ├── ffi.rs            ← function pointer table (SpikeFns / NzeaFns)
│   └── lib.rs            ← mod_flat!(ffi, runtime, ...)
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

### 4. Loading with libloading

Use a static `OnceLock<Result<Fns, String>>` to cache the loaded library.
Define a function pointer struct and load symbols with `lib.get()` + transmute.

```rust
static LIBS: OnceLock<Result<MyFns, String>> = OnceLock::new();

pub(crate) fn ensure_loaded() -> Result<&'static MyFns, String> {
    LIBS.get_or_init(|| {
        if !fresh { build_so()?; }
        let lib = unsafe { Library::new(&so_path())? };
        MyFns::from_library(lib)
    }).as_ref().map_err(|e| e.clone())
}
```

## FFI Module (`src/ffi.rs`)

Replace `extern "C"` blocks with a function pointer struct loaded via libloading:

```rust
pub(crate) struct MyFns {
    pub init: unsafe extern "C" fn(...) -> *mut c_void,
    pub step: unsafe extern "C" fn(...) -> c_int,
}
impl MyFns {
    pub(crate) fn from_library(lib: Library) -> Result<Self, String> { /* ... */ }
}
```

## Reference implementations

- `remu_simulator/simulators/spike/src/runtime.rs` — single target
- `remu_simulator/simulators/nzea/src/runtime.rs` — per-(platform, isa) combination
