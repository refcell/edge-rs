---
title: Data Locations
---

# Data Locations

```text
<storage_pointer> ::= "&s" ;
<transient_storage_pointer> ::= "&t" ;
<memory_pointer> ::= "&m" ;
<calldata_pointer> ::= "&cd" ;
<returndata_pointer> ::= "&rd" ;
<internal_code_pointer> ::= "&ic" ;
<external_code_pointer> ::= "&ec" ;

<data_location> ::=
    | <storage_pointer>
    | <transient_storage_pointer>
    | <memory_pointer>
    | <calldata_pointer>
    | <returndata_pointer>
    | <internal_code_pointer>
    | <external_code_pointer> ;
```

The `<location>` is a data location annotation indicating to which data
location a pointer's data exists. We define seven distinct annotations
for data location pointers. This is a divergence from general purpose
programming languages to more accurately represent the EVM execution
environment.

* `&s` persistent storage
* `&t` transient storage
* `&m` memory
* `&cd` calldata
* `&rd` returndata
* `&ic` internal (local) code
* `&ec` external code

## Semantics

Data locations can be grouped into two broad categories, buffers and maps.

### Maps

Persistent and transient storage are part of the map category,
256 bit keys map to 256 bit values. Both may be written or read one
word at a time.

### Buffers

Memory, calldata, returndata, internal code, and external code are
all linear data buffers. All can be either read to the stack or copied
into memory, but only memory can be written or copied to.

| Name          | Read to Stack | Copy to Memory | Write |
|---------------|---------------|----------------|-------|
| memory        | true          | true           | true  |
| calldata      | true          | true           | false |
| returndata    | false         | true           | false |
| internal code | false         | true           | false |
| external code | false         | true           | false |

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

### Pointer Bit Sizes

Pointers to different data locations consist of different sizes based on
the properties of that data location. In depth semantics of each data
location are specified in the type system documents.

| Location           |	Bit Size |	Reason                                                       |
| -------------------|-----------|---------------------------------------------------------------|
| persistent storage | 256       | Storage is 256 bit key value hashmap                          |
| transient storage  | 256       | Transient storage is 256 bit key value hashmap                |
| memory             | 32        | Theoretical maximum memory size does not grow to 0xffffffff   |
| calldata           | 32        | Theoretical maximum calldata size does not grow to 0xffffffff |
| returndata         | 32        | Maximum returndata size is equal to maximum memory size       |
| internal code      | 16        | Code size is less than 0xffff                                 |
| external code      | 176       | Contains 160 bit address and 16 bit code pointer              |
