---
title: Data locations
---

# Data locations

```text
<storage_pointer> ::= "&s" ;
<transient_storage_pointer> ::= "&t" ;
<memory_pointer> ::= "&m" ;
<dynamic_memory_pointer> ::= "&dm" ;
<calldata_pointer> ::= "&cd" ;
<returndata_pointer> ::= "&rd" ;
<internal_code_pointer> ::= "&ic" ;
<external_code_pointer> ::= "&ec" ;

<data_location> ::=
    | <storage_pointer>
    | <transient_storage_pointer>
    | <memory_pointer>
    | <dynamic_memory_pointer>
    | <calldata_pointer>
    | <returndata_pointer>
    | <internal_code_pointer>
    | <external_code_pointer> ;
```

The `<data_location>` is a pointer annotation indicating which EVM data region
a value resides in. Edge defines eight distinct location annotations. This is a
divergence from general-purpose programming languages to more accurately represent
the EVM execution environment.

* `&s` — persistent storage
* `&t` — transient storage (EIP-1153)
* `&m` — memory
* `&dm` — dynamic memory
* `&cd` — calldata
* `&rd` — returndata
* `&ic` — internal (local) code
* `&ec` — external code

:::note
The `&` character is heavily overloaded in the lexer. It checks for data-location
sigils first (`&s`, `&t`, `&m`, `&dm`, `&cd`, `&rd`, `&ic`, `&ec`), then `&=`, then
`&&`, and finally falls back to bitwise AND.
:::

## Semantics

Data locations can be grouped into two broad categories: buffers and maps.

### Maps

Persistent and transient storage are part of the map category —
256-bit keys map to 256-bit values. Both may be written or read one
word at a time.

### Dynamic memory

The `&dm` annotation designates a pointer to **dynamically allocated
memory**. Unlike `&m` (which refers to compiler-managed fixed-offset
memory), `&dm` pointers are obtained at runtime via the `@alloc` builtin
and point to MSIZE-allocated regions.

`&dm` enables:

- **Runtime-sized allocations**: `@alloc(size_bytes)` returns a pointer
  to a fresh memory region that does not overlap with any other allocation.
- **Transparent pointer aliasing**: When a method takes `self: &dm Self`,
  mutations to the underlying pointer (e.g., during `Vec` growth and
  reallocation) are propagated back to the caller transparently.
- **Pass-by-reference semantics**: Methods on `&dm` types receive and
  can update the caller's pointer, enabling in-place mutation of
  dynamically-sized structures.

`&dm` is primarily used by `Vec<T>` and other dynamically-sized standard
library types. The compiler tracks `&dm` regions symbolically for
optimization (region-based store forwarding, non-aliasing proofs).

```edge
type Vec<T> = { len: u256, capacity: u256 };

impl Vec<T> {
    fn len(self: &dm Self) -> u256 {
        self.len
    }

    fn push(self: &dm Self, val: T) {
        // self is a &dm pointer — mutations to self.len
        // are visible to the caller
        // ...
    }
}
```

### Buffers

Memory, calldata, returndata, internal code, and external code are
all linear data buffers. All can be either read to the stack or copied
into memory, but only memory can be written or copied to.

| Name          | Read to stack | Copy to memory | Write |
|---------------|---------------|----------------|-------|
| memory        | yes           | yes            | yes   |
| calldata      | yes           | yes            | no    |
| returndata    | no            | yes            | no    |
| internal code | no            | yes            | no    |
| external code | no            | yes            | no    |

### Transitions

Transitioning from map to memory buffer is performed by loading each
element from the map to the stack and storing each stack item in memory
O(N).

Transitioning from memory buffer to a map is performed by loading each
element from memory to the stack and storing each stack item in the map
O(N).

Transitioning from any other buffer to a map is performed by copying the
buffer's data into memory then transitioning the data from memory into the
map O(N+1).

### Pointer bit sizes

Pointers to different data locations consist of different sizes based on
the properties of that data location. In-depth semantics of each data
location are specified in the type system documents.

| Location           | Bit size | Reason |
|--------------------|----------|--------|
| persistent storage | 256 | Storage is a 256-bit key–value hashmap |
| transient storage  | 256 | Transient storage is a 256-bit key–value hashmap |
| memory             | 32  | Theoretical maximum memory size does not approach 2³² |
| dynamic memory     | 32  | Theoretical maximum memory size does not approach 2³² |
| calldata           | 32  | Theoretical maximum calldata size does not approach 2³² |
| returndata         | 32  | Maximum returndata size equals maximum memory size |
| internal code      | 16  | Code size is less than 0xFFFF |
| external code      | 176 | Contains 160-bit address and 16-bit code pointer |
