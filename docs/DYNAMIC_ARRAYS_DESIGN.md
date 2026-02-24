# Dynamic Arrays Design (Pathfinder)

## Purpose

Define and compare multiple implementation strategies for dynamic arrays in DNA VisiCalc, then record the selected implementation for this round.

## Public reference anchors (Excel behavior)

- Microsoft support: dynamic arrays and spilled array behavior (`#SPILL!`, spill ranges, resize behavior).
  - https://support.microsoft.com/en-us/office/dynamic-array-formulas-and-spilled-array-behavior
- Microsoft support: spilled range operator (`#`) and dynamic array references.
  - https://support.microsoft.com/en-us/office/spilled-range-operator
- Microsoft support: `SEQUENCE`.
  - https://support.microsoft.com/en-us/office/sequence-function
- Microsoft support: `RANDARRAY`.
  - https://support.microsoft.com/en-us/office/randarray-function

This pathfinder intentionally implements a smaller subset than Excel, while matching key conceptual behavior.

## Scope implemented now

- Formula results can be scalar or dynamic arrays.
- Dynamic array output spills from anchor cell to a rectangular spill range.
- Spill references with `A1#`.
- `#SPILL` errors for blocked/out-of-bounds spill.
- `#REF`-style error semantics for invalid spill references.
- Element-wise arrayification for binary operators (scalar/array broadcasting or same-shape arrays).
- `SEQUENCE` and `RANDARRAY`.
- TUI spill affordances:
  - spill anchor/member visual hints,
  - edit constraints on spill children,
  - context spill-range indicator.

## Three architecture options

### Option A: Spill Overlay on Existing Scalar Engine (implemented)

- Keep existing formula graph and evaluation pipeline.
- Extend expression runtime value from scalar-only to scalar-or-array.
- After formula evaluation, run a spill-placement phase:
  - validate bounds/conflicts,
  - assign spill ownership,
  - write cell values for anchor + spill children.

Pros:
- Minimal disruption to existing engine API and tests.
- Preserves deterministic calc ordering.
- Easy to introduce incrementally.

Cons:
- Spill state is a derived overlay, not a first-class graph object.
- Some advanced behaviors (full array dependency tracing) remain future work.

### Option B: Array-First Value Graph

- Make every node value first-class array (1x1 for scalar).
- Dependency graph and operators are array-native.
- Projection to cells becomes a rendering/output step.

Pros:
- Cleaner long-term semantics.
- Strong fit for automatic arrayification and richer broadcasting rules.

Cons:
- Larger rearchitecture.
- Requires broad API and test refactor.

Prototype:
- `crates/dnavisicalc-core/src/experiments/array_graph.rs`

### Option C: Rewrite/Desugar Dynamic Arrays to Scalar Cells

- Expand dynamic formulas into generated scalar formulas over target range.
- Treat generated formulas as normal scalar formulas.

Pros:
- Reuses mature scalar execution path.
- Spill behavior can be explicit in generated artifacts.

Cons:
- Rewrite lifecycle complexity (updates, invalidation, provenance).
- Harder to preserve source-level intent/debuggability.

Prototype:
- `crates/dnavisicalc-core/src/experiments/spill_rewrite.rs`

## Why Option A now

- It keeps current pathfinder momentum while adding real dynamic-array capability.
- It is compatible with existing core test corpus and layering.
- It leaves clear seams for migration toward Option B later if needed.

## Implemented experimental modules

- Overlay planner prototype:
  - `crates/dnavisicalc-core/src/experiments/spill_overlay.rs`
- Array-first graph prototype:
  - `crates/dnavisicalc-core/src/experiments/array_graph.rs`
- Rewrite/desugar prototype:
  - `crates/dnavisicalc-core/src/experiments/spill_rewrite.rs`

These are intentionally isolated from production engine flow, but compile and test as concrete experiments.

## Known limits and open questions

- Full Excel parity for all dynamic-array interactions is out of scope.
- Current array broadcasting rules are intentionally simple.
- `RANDARRAY` is deterministic per recalc pass in this implementation; volatility policy can evolve by profile.
- Advanced functions and text-array behavior are deferred.

## Test strategy updates

- Added dedicated core tests for:
  - spill placement/metadata,
  - blocked spill errors,
  - spill reference behavior (`A1#`),
  - arrayified binary operations,
  - `RANDARRAY` bounds and shape.
- Added TUI tests for:
  - non-editable spill child cells,
  - spill anchor/member grid role marking.
