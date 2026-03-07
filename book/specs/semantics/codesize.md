# Codesize

This document details the different options for codesize optimization. Generally,
codesize and runtime efficiency are inversely correlated. Developers will have
granular control both in the compiler's configuration and in the language's syntax.

## Inlining Heuristics

Function inlining is a direct tradeoff of codesize and runtime efficiency.
Codesize optimization may be used for reducing deployment cost or for keeping
the codesize below the EVM's codesize limit.

## Scoring

Functions are assigned a score based on a combination of its projected bytecode
size, projected number of calls, and an optional manually entered score.

| Name          | Score                                        |
|---------------|----------------------------------------------|
| Bytecode Size	| `fn.bytecode.len()`                          |
| Call Count    | `fn.calls()`                                 |
| Manual Score  | `u8`                                         |
| Total         | `(fn.bytecode.len() + 5 * fn.calls()) * man` |

> todo rewrite this based on gas estimations of each call

A compiler configuration can be specified for the threshold for function inlining.

> todo decide on this

## Analysis

The analysis for function inline scoring requires the traveral of a directed
graph containing each function and other functions called within it. Traversal
is depth first, as function inline scores are dependent on their bytecode size
which is dependent on the inline scores of functions called within its body.
Once a terminal function, a function with no internal function dependencies,
is found, its inline score will be compared against the configuration threshold.
If the score is greater than the threshold, it is to be inlined and a flag will
be stored in the graph for future references.

Cycle detection will both prevent infinite loops in the compiler as well as
detect recursion and corecursion. Recursive and corecursive functions will
never be inlined for simplicity.

## Dead Code Elimination

Eliminating dead code will cut codesize and improve the function inlining score,
as number of calls and projected codesize of each function are both factors in
the function inline score.

## Syntax Modifications

> todo
