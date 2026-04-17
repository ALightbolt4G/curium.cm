# Curium Standard Library Documentation

The Curium Standard Library (`std`) provides core utilities for file systems, process control, dynamic collections, and formatting.

## Core Modules

### `std/process`
Process management, program arguments, and exit controls.

- `fn args() -> []string`: Returns a dynamic array of command-line arguments passed to the program.
- `fn env_argc() -> i32`: Returns the number of arguments passed via `__curium_argc`.
- `fn process_exit(code: i32) -> void`: Instantly terminates the running process with the given return code.

### `std/vec`
Dynamic, resizable Collections relying heavily on the dynamic runtime.

- `struct Vector`: A wrapper around an array of `curium_dyn_t` dynamic sizes.
- `fn vec_new() -> ^Vector`: Allocates an empty `Vector` of initial capacity 4 under the reference-counted memory system.
- `fn vec_push(v: ^Vector, element: dyn) -> void`: Takes a generic `dyn` structure and appends it dynamically to the Vector via reallocation if capacity is reached.
- `fn vec_len(v: ^Vector) -> i32`: Safely retrieves the populated length.
- `fn vec_get(v: ^Vector, index: i32) -> dyn`: Bound-checked element retrieval, returning `DYN_NULL` out of bounds.

*(Helper Functions)*
- `fn dyn_int(n: i32) -> dyn`: Helper function used to instantiate dynamic boxes over primitives.

### `std/string`
Heap-allocated dynamic strings backed by the `curium_string_t` type.

- `fn concat(a: string, b: string) -> string`: Appends `b` after `a` returning an independent string.
- `fn int_to_string(n: i32) -> string`: Formats integers dynamically via C-native formatting (`snprintf`).

### `std/fs`
Basic POSIX file I/O operations natively wrapped behind Polyglot C Blocks.

- `fn write_string(path: string, content: string) -> i32`: Writes strings, returning `1` for success and `0` for failures.
- `fn read_to_string(path: string) -> string`: Allocates the full physical size of a generic file.
