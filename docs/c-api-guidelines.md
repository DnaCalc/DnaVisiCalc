# C API Design Guidelines

Guidelines for writing C APIs of lasting quality — distilled from the best interfaces of the past three decades.

## 1. Principles

A great C API is discovered, not learned. When a programmer encounters it for the first time, naming alone should reveal intent; reading the header should be sufficient to use the library without a manual. These principles guide that outcome:

**Stability over convenience.** A released function signature is a permanent contract. Design for the interface you will maintain for a decade, not the one that is easiest to implement today. Every parameter, every return value, every type name becomes a promise.

**Discoverability through consistency.** When every function follows the same naming and parameter conventions, the programmer can guess the next function's signature before seeing it. Patterns substitute for documentation.

**Self-documenting headers.** The `.h` file is the specification. Types, function signatures, and enum values should encode enough information that the programmer rarely needs prose. Comments in headers clarify intent and contracts — never compensate for poor naming.

**Explicit resource ownership.** Every allocation has a visible creator and a visible destroyer. The programmer never wonders "who frees this?" because the answer is encoded in the function names and parameter conventions.

**Zero hidden global state.** Thread safety, reentrancy, and testability all follow from one rule: the library holds no global mutable state. All state lives behind explicit handles. Multiple independent instances coexist without interference.

## 2. Naming Conventions

### The `prefix_noun_verb` Pattern

Every public symbol begins with a short, unique prefix (typically 2–5 characters). After the prefix comes the noun (the resource being operated on), then the verb (the action). This produces a natural reading order and groups related functions together in sorted listings.

```c
/* SQLite: prefix = sqlite3_ */
sqlite3_open()           /* noun=db(implicit)  verb=open    */
sqlite3_prepare_v2()     /* noun=statement     verb=prepare */
sqlite3_step()           /* noun=statement     verb=step    */
sqlite3_finalize()       /* noun=statement     verb=finalize*/
sqlite3_close()          /* noun=db            verb=close   */

/* Vulkan: prefix = vk */
vkCreateDevice()         /* noun=Device   verb=Create  */
vkDestroyDevice()        /* noun=Device   verb=Destroy */
vkAllocateMemory()       /* noun=Memory   verb=Allocate*/
vkFreeMemory()           /* noun=Memory   verb=Free    */

/* libuv: prefix = uv_ */
uv_loop_init()           /* noun=loop  verb=init  */
uv_loop_close()          /* noun=loop  verb=close */
uv_tcp_bind()            /* noun=tcp   verb=bind  */
uv_read_start()          /* noun=read  verb=start */

/* Windows NT: prefix = Nt/Zw */
NtCreateFile()           /* noun=File    verb=Create */
NtQueryInformationFile() /* noun=InformationFile verb=Query */
NtClose()                /* noun=Handle  verb=Close  */
```

### Create/Destroy Pairs

Every resource has a matched pair. The naming makes the pairing unmistakable:

| Create | Destroy | API |
|--------|---------|-----|
| `sqlite3_open` | `sqlite3_close` | SQLite |
| `vkCreateInstance` | `vkDestroyInstance` | Vulkan |
| `uv_loop_init` | `uv_loop_close` | libuv |
| `curl_easy_init` | `curl_easy_cleanup` | libcurl |
| `lua_newstate` | `lua_close` | Lua |
| `CreateFile` | `CloseHandle` | Win32 |

Pick one verb pair and use it everywhere: `create`/`destroy`, `init`/`close`, `new`/`free`. Mixing pairs within a single API (e.g. `open` in one place, `create` in another, `init` in a third) destroys discoverability.

### Consistent Prefixes

The prefix is the namespace. It must be:
- **Short** — 2–5 lowercase characters, or PascalCase for Win32-style APIs.
- **Unique** — no collision with standard library or common dependency symbols.
- **Universal** — applied to every public symbol: functions, types, enums, macros.

```c
/* Good: consistent prefix */
typedef struct DvcEngine DvcEngine;
DvcStatus dvc_engine_create(DvcEngine **out);
DvcStatus dvc_engine_destroy(DvcEngine *engine);
DvcStatus dvc_cell_set_number(DvcEngine *engine, DvcCellAddr addr, double value);

/* Bad: inconsistent or missing prefix */
Engine *create_engine(void);         /* no prefix */
int engine_set_cell(Engine *e, ...); /* different prefix */
void destroyEng(Engine *e);          /* abbreviated differently */
```

## 3. Handle and Lifetime Patterns

### Opaque Handles

The handle is a pointer to an incomplete (forward-declared) struct. The consumer sees only the pointer; the implementation defines the struct privately.

```c
/* Public header */
typedef struct sqlite3 sqlite3;
typedef struct sqlite3_stmt sqlite3_stmt;

int sqlite3_open(const char *filename, sqlite3 **ppDb);
int sqlite3_close(sqlite3 *db);
int sqlite3_prepare_v2(sqlite3 *db, const char *sql, int nByte,
                       sqlite3_stmt **ppStmt, const char **pzTail);
int sqlite3_finalize(sqlite3_stmt *pStmt);
```

This pattern achieves:
- **ABI stability** — the struct's layout can change without recompiling consumers.
- **Encapsulation** — the consumer cannot reach into internal fields.
- **Clear ownership** — the handle is the single token of ownership.

### Explicit Create/Destroy

Every handle-producing function has a corresponding destroyer. The contract is simple: if `create` succeeds, the caller *must* eventually call `destroy`. If `create` fails, no handle exists and no cleanup is needed.

```c
/* Vulkan pattern: creation info struct + explicit allocator */
VkResult vkCreateInstance(
    const VkInstanceCreateInfo *pCreateInfo,
    const VkAllocationCallbacks *pAllocator,  /* optional */
    VkInstance *pInstance                      /* out */
);

void vkDestroyInstance(
    VkInstance instance,
    const VkAllocationCallbacks *pAllocator   /* must match create */
);
```

### No Hidden Shared Ownership

A handle belongs to exactly one owner at a time. The library never secretly retains a reference to a handle the caller has destroyed. If a parent handle owns child handles, destruction order is explicit and documented:

> "All `sqlite3_stmt` objects must be finalized before calling `sqlite3_close` on the parent `sqlite3` connection."

This is the SQLite model. The simpler alternative (parent destroy implicitly destroys children) trades safety for convenience — choose one and document it.

## 4. Error Handling

### Integer Status Codes

The primary error signal is an integer return value. Zero means success. Non-zero means failure. This is universal across C APIs of quality.

```c
/* SQLite */
#define SQLITE_OK       0
#define SQLITE_ERROR    1
#define SQLITE_BUSY     5
#define SQLITE_NOMEM    7

/* Vulkan */
typedef enum VkResult {
    VK_SUCCESS = 0,
    VK_NOT_READY = 1,           /* positive = non-error status */
    VK_ERROR_OUT_OF_HOST_MEMORY = -1,
    VK_ERROR_OUT_OF_DEVICE_MEMORY = -2,
    /* ... */
} VkResult;

/* NT kernel */
typedef LONG NTSTATUS;
#define STATUS_SUCCESS                0x00000000
#define STATUS_BUFFER_TOO_SMALL      0xC0000023
#define STATUS_INVALID_PARAMETER     0xC000000D
```

The Vulkan/NTSTATUS convention — negative values for errors, zero for success, positive for non-error status — is the most expressive. The SQLite convention — zero for success, all non-zero for errors — is simpler and sufficient for most APIs.

### Per-Handle Error Detail

Status codes identify the *category* of failure. Detailed messages require a separate channel. Two proven approaches:

**Per-handle error buffer** (SQLite model):
```c
const char *sqlite3_errmsg(sqlite3 *db);
/* Returns human-readable text for the most recent error on this handle. */
/* Thread-safe: each handle has its own error state. */
```

**Thread-local error** (Win32 model):
```c
DWORD GetLastError(void);
/* Returns error code for the calling thread's last failed API call. */
```

The per-handle model is strongly preferred for modern APIs. Thread-local error state is fragile — any intervening call can overwrite it. Per-handle state is deterministic and safe under any threading model.

### What to Avoid

- **errno** — global, overwritten by any libc call, meaningless across threads without careful discipline.
- **Exception-like longjmp** — destroys resource cleanup invariants.
- **Output-only error parameter** — forces the caller to check a pointer instead of a return value; easy to ignore.

## 5. Memory Management

### Caller-Provided Buffers

When the library needs to return variable-length data (strings, arrays), the cleanest pattern is caller-provided buffers with explicit length negotiation.

```c
/* Pattern: call once with NULL to query size, then call again with buffer */

/* NT kernel style */
NTSTATUS NtQueryInformationFile(
    HANDLE FileHandle,
    PIO_STATUS_BLOCK IoStatusBlock,
    PVOID FileInformation,      /* caller-allocated buffer */
    ULONG Length,               /* buffer size in bytes */
    FILE_INFORMATION_CLASS FileInformationClass
);

/* Simplified for a string return */
DvcStatus dvc_cell_get_text(
    DvcEngine *engine,
    DvcCellAddr addr,
    char *buf,          /* caller buffer, or NULL to query length */
    uint32_t buf_len,   /* buffer capacity in bytes */
    uint32_t *out_len   /* actual length written (or required) */
);
```

The caller controls all allocation. The library never calls `malloc` on the caller's behalf (or if it does, the contract is explicit and the caller frees with a library-provided function).

### Custom Allocator Callbacks

For libraries that must allocate internally (complex state, caches), an allocator callback table lets the caller control all heap operations:

```c
/* Vulkan allocator callbacks */
typedef struct VkAllocationCallbacks {
    void *pUserData;
    PFN_vkAllocationFunction pfnAllocation;
    PFN_vkReallocationFunction pfnReallocation;
    PFN_vkFreeFunction pfnFree;
    /* ... */
} VkAllocationCallbacks;

/* SQLite custom allocator */
sqlite3_config(SQLITE_CONFIG_MALLOC, &custom_methods);
```

This pattern is essential for embedded systems, game engines, and any environment where the default allocator is unsuitable.

### Rules

1. Every byte the library allocates must be freeable through a library-provided function or a caller-provided allocator.
2. Buffer sizes are always in bytes (not elements, not characters) unless the API is exclusively for a fixed-width type.
3. Output length parameters report the *required* size including any null terminator.
4. A buffer of size zero with a non-NULL out_len pointer is a size query, never an error.

## 6. Versioned Extensibility

### Struct-Size Versioning

The simplest forward-compatible pattern: every configuration/info struct begins with a size field.

```c
typedef struct DvcEngineCreateInfo {
    uint32_t struct_size;    /* = sizeof(DvcEngineCreateInfo) */
    uint16_t max_columns;
    uint16_t max_rows;
} DvcEngineCreateInfo;

DvcStatus dvc_engine_create(const DvcEngineCreateInfo *info, DvcEngine **out);
```

When the struct grows in a future version, old callers pass a smaller `struct_size`. The library detects this and applies defaults for new fields. New callers pass the larger size and populate new fields. No version enum is needed — the size *is* the version.

### Vulkan-Style pNext Chains

For open-ended extensibility, Vulkan's `sType` + `pNext` pattern allows arbitrary extension structs to be chained:

```c
typedef struct VkInstanceCreateInfo {
    VkStructureType sType;    /* = VK_STRUCTURE_TYPE_INSTANCE_CREATE_INFO */
    const void *pNext;        /* pointer to extension struct, or NULL */
    /* ... base fields ... */
} VkInstanceCreateInfo;

typedef struct VkDebugUtilsMessengerCreateInfoEXT {
    VkStructureType sType;    /* = VK_STRUCTURE_TYPE_DEBUG_UTILS_MESSENGER_CREATE_INFO_EXT */
    const void *pNext;
    /* ... extension fields ... */
} VkDebugUtilsMessengerCreateInfoEXT;
```

This is powerful but complex. Use `struct_size` for simple APIs; reserve `pNext` chains for APIs that genuinely need open-ended extension by third parties.

### API Version Query

Every library should expose a compile-time and runtime version:

```c
/* Compile-time (header) */
#define DVC_API_VERSION_MAJOR 0
#define DVC_API_VERSION_MINOR 3
#define DVC_API_VERSION_PATCH 0

/* Runtime (linked library) */
uint32_t dvc_api_version(void);  /* packed: (major << 16) | (minor << 8) | patch */
```

This lets consumers detect version skew between headers and linked library.

## 7. Thread Safety Documentation

### Per-Function Contracts

Every function's documentation must state its thread safety guarantee explicitly. The three levels:

1. **Thread-safe (global)** — may be called concurrently from any thread with any arguments. (Rare for mutable operations.)
2. **Thread-safe (per-handle)** — may be called concurrently on *different* handles. Concurrent calls on the *same* handle require external synchronization.
3. **Not thread-safe** — caller must serialize all calls.

Most high-quality C APIs choose level 2: distinct handles are independent, but a single handle is not internally synchronized.

```
/* SQLite documentation example */
/*
 * sqlite3_step() is safe to call from multiple threads on different
 * sqlite3_stmt handles. Calling sqlite3_step() concurrently on the
 * same sqlite3_stmt is undefined behavior.
 */
```

### No Hidden Global State

The strongest thread safety guarantee comes from eliminating global state entirely:

- No global initialization functions (or if needed, make them idempotent and thread-safe).
- No global configuration that affects existing handles.
- Per-handle state only.

```c
/* Good: per-handle configuration */
dvc_engine_set_recalc_mode(engine, DVC_RECALC_MANUAL);

/* Bad: global configuration */
dvc_set_default_recalc_mode(DVC_RECALC_MANUAL);  /* affects all future handles? existing? */
```

### Callback Threading Contracts

If the library invokes user-provided callbacks, document which thread the callback runs on and what operations are safe inside the callback:

> "The `uv_read_cb` callback is always invoked on the event loop thread. It is safe to call `uv_read_stop` from within the callback. Calling `uv_loop_close` from within the callback is undefined behavior."

## 8. Header File Organization

### Structure

```c
#ifndef DVC_API_H
#define DVC_API_H

#ifdef __cplusplus
extern "C" {
#endif

#include <stdint.h>
#include <stddef.h>

/* ── Version ────────────────────────────────────────────── */
#define DVC_API_VERSION_MAJOR 0
#define DVC_API_VERSION_MINOR 3

/* ── Status Codes ───────────────────────────────────────── */
typedef int32_t DvcStatus;
#define DVC_OK                    0
#define DVC_ERR_INVALID_ARGUMENT -1
/* ... */

/* ── Opaque Handles ─────────────────────────────────────── */
typedef struct DvcEngine DvcEngine;

/* ── Value Types and Enumerations ───────────────────────── */
typedef int32_t DvcValueType;
#define DVC_VALUE_NUMBER  0
#define DVC_VALUE_TEXT    1
/* ... */

/* ── Data Structures ────────────────────────────────────── */
typedef struct { uint16_t col; uint16_t row; } DvcCellAddr;
/* ... */

/* ── Lifecycle Functions ────────────────────────────────── */
DvcStatus dvc_engine_create(DvcEngine **out);
DvcStatus dvc_engine_destroy(DvcEngine *engine);

/* ── Cell Functions ─────────────────────────────────────── */
DvcStatus dvc_cell_set_number(DvcEngine *engine, DvcCellAddr addr, double value);
/* ... */

#ifdef __cplusplus
}
#endif

#endif /* DVC_API_H */
```

### Rules

1. **Include guard** — `#ifndef` / `#define` / `#endif`, or `#pragma once` where portability allows.
2. **`extern "C"` wrapper** — always present so the header works from C++.
3. **Minimal includes** — only `<stdint.h>` and `<stddef.h>`. Never pull in platform headers.
4. **Types before functions** — all typedefs and structs appear before any function declaration.
5. **Grouped by resource** — all functions operating on the same handle type are adjacent.
6. **No implementation details** — no struct definitions for opaque types in the public header.

## 9. Complete Example

A minimal counter API demonstrating every pattern together:

```c
/* cntr_api.h — Counter API, demonstrating C API design patterns */
#ifndef CNTR_API_H
#define CNTR_API_H

#ifdef __cplusplus
extern "C" {
#endif

#include <stdint.h>

/* ── Version ────────────────────────────────────────────── */
#define CNTR_API_VERSION_MAJOR 1
#define CNTR_API_VERSION_MINOR 0

uint32_t cntr_api_version(void);

/* ── Status Codes ───────────────────────────────────────── */
typedef int32_t CntrStatus;
#define CNTR_OK               0
#define CNTR_ERR_NULL_POINTER -1
#define CNTR_ERR_OVERFLOW     -2

/* ── Opaque Handle ──────────────────────────────────────── */
typedef struct CntrCounter CntrCounter;

/* ── Configuration ──────────────────────────────────────── */
typedef struct CntrCounterCreateInfo {
    uint32_t struct_size;   /* = sizeof(CntrCounterCreateInfo) */
    int64_t  initial_value;
    int64_t  step;          /* increment per advance; default 1 */
} CntrCounterCreateInfo;

/* ── Lifecycle ──────────────────────────────────────────── */

/*
 * Create a new counter. On success, *out receives the handle.
 * Thread safety: safe to call concurrently (no shared state).
 */
CntrStatus cntr_counter_create(const CntrCounterCreateInfo *info,
                                CntrCounter **out);

/*
 * Destroy a counter and release all resources.
 * Passing NULL is a safe no-op.
 * Thread safety: must not be called concurrently with other
 * operations on the same handle.
 */
CntrStatus cntr_counter_destroy(CntrCounter *counter);

/* ── Operations ─────────────────────────────────────────── */

/*
 * Advance the counter by its step value.
 * Returns CNTR_ERR_OVERFLOW if the result would exceed int64 range.
 * Thread safety: per-handle; concurrent calls on different handles are safe.
 */
CntrStatus cntr_counter_advance(CntrCounter *counter);

/*
 * Read the current counter value.
 * Thread safety: per-handle.
 */
CntrStatus cntr_counter_value(const CntrCounter *counter, int64_t *out);

/* ── Error Detail ───────────────────────────────────────── */

/*
 * Return a human-readable error message for the last failed operation
 * on this handle. The returned pointer is valid until the next API call
 * on the same handle. Returns "" if no error has occurred.
 * Thread safety: per-handle.
 */
const char *cntr_last_error_message(const CntrCounter *counter);

#ifdef __cplusplus
}
#endif

#endif /* CNTR_API_H */
```

This 60-line header demonstrates:
- Consistent `cntr_` prefix
- Opaque handle (`CntrCounter`)
- Versioned create-info struct with `struct_size`
- Matched `create`/`destroy` pair
- Integer status codes (zero = success, negative = error)
- Per-handle error detail buffer
- `const` correctness on read-only operations
- Per-function thread safety documentation
- `extern "C"` wrapper
- Minimal includes

## References

| API | Key quality extracted |
|-----|---------------------|
| **SQLite** | Handle lifecycle discipline (`sqlite3*` → `sqlite3_stmt*` → `finalize` → `close`); per-handle error messages; minimal footprint |
| **Vulkan** | Explicit ownership encoded in names (`Create`/`Destroy`); allocator callback hooks; versioned info structs with `sType` + `pNext` chains |
| **libuv** | Event callback model; `loop` / `handle` / `request` hierarchy; consistent `uv_` prefix across all resource types |
| **libcurl** | `easy` / `multi` interface split; `CURLOPT_*` configuration by enum + value; progressive complexity (simple things simple, complex things possible) |
| **Lua** | Stack-based embedding boundary; `lua_State*` as the single handle; clean C89 compatibility |
| **Windows NT kernel** | Consistent naming discipline across thousands of functions; handle-based resource model; structured status codes (NTSTATUS with severity/facility/code bit fields); information classes for extensible queries; caller-provided buffers with length semantics |
