# edge-diagnostics

Error reporting and diagnostic infrastructure for the Edge compiler. Provides structured diagnostics with severity levels, source labels, and pretty-printed error output.

## Pipeline Position

```
source -> lexer -> parser -> AST -> typeck -> IR -> codegen -> driver -> bytecode
           \________\_________\________\________\________\________/
                        diagnostics (used by all phases)
```

## What It Does

- Defines `Diagnostic` values with severity (error, warning, note), a message, source labels, and notes
- Labels point to `Span` locations in source code for precise error highlighting
- `DiagnosticBag` accumulates diagnostics across compilation phases
- `report_all()` renders diagnostics to stderr with line numbers, carets, and context

## Key Types

- **`Diagnostic`** -- A single compiler diagnostic with severity, message, labels, and notes
- **`Severity`** -- `Error`, `Warning`, or `Note`
- **`Label`** -- Points to a `Span` in source with a message and severity
- **`DiagnosticBag`** -- Collects diagnostics; provides `has_errors()`, `error_count()`, `warning_count()`

## Usage

```rust,no_run
use edge_diagnostics::{Diagnostic, DiagnosticBag};
use edge_types::span::Span;

let mut bag = DiagnosticBag::new();

bag.push(
    Diagnostic::error("type mismatch")
        .with_label(Span { start: 10, end: 15 }, "expected u256")
        .with_note("did you mean to use a cast?"),
);

if bag.has_errors() {
    bag.report_all(source);
}
```

## Example Output

```
error: type mismatch
  --> line 3:5
   |
   | let x: u8 = 256;
   |              ^^^ expected u256
   |
   = note: did you mean to use a cast?
```

## Integration

- **Used by**: `edge-driver` (and potentially all compiler phases)
- **Dependencies**: `edge-types` (for `Span`)
