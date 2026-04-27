# llzk-spec context

Below is a markdown version of the latest plan for the llzk-spec language

## Purpose

The purpose of this document is to provide a V1 design of the high-level specification language for LLZK as proposed as part of the LLZK Maintenance Grant. The original proposal is found here: [LLZK Specifications](https://www.notion.so/LLZK-Specifications-2b0105edf1db80108d04d9e5f0452c00?pvs=21).

<aside>
💡

Unless we come up with a better name, we’ll refer to this as llzk-spec.

</aside>

## Language Goals and Non-Goals

### Goals

- llzk-spec is designed to target generated LLZK IR compiled from source ZK DSLs.
    - The symbols referenced in llzk-spec must therefore correspond to symbols in the LLZK IR, which may different slightly from the source language naming (e.g., concrete mode-compiled

### Non-Goals

- llzk-spec will not target arbitrary language constructs that do not have a corollary in the LLZK IR.
- llzk-spec will not replace any specification languages that are native to source ZK DSLs.
    - The ZK DSL in question could either emit `verif` dialect specs directly when compiled or could

## Deliverables

For Milestone 1 of the LLZK Maintenance Grant, we have committed to delivering the following:

- High-level specification language implementation
- Support for writing specs for ZK DSLs without native specification support
- Example specifications demonstrating usage in LLZK

## Current Implementation Context

The repository now contains a working Phase 1 `llzk-spec` compiler implemented as a standalone Rust crate.

- The parser is generated with `pest`, using the grammar at [./src/grammar/llzk_spec.pest](./src/grammar/llzk_spec.pest).
- The compiler pipeline is currently:
  - parse a `.spec` file into a custom AST
  - load LLZK IR and extract symbol metadata by walking parsed MLIR operations
  - verify spec references against that metadata
  - optionally emit the AST in `debug` or `json` form
- The project intentionally uses its own `Diagnostic` type as the public/user-facing diagnostic representation.
  - Parser, verifier, and IR-loading stages all normalize failures into this shared format.
  - Raw MLIR diagnostics are treated as an internal source of detail, not as the compiler’s public diagnostic API.
- End-to-end coverage lives in cargo-integrated lit-style Rust integration tests.
  - `tests/lit.rs` is the Rust harness that discovers and runs `tests/lit/**/*.spec`.
  - LLZK IR inputs for those tests are stored under [./tests/lit/Inputs](./tests/lit/Inputs).
  - Current CLI coverage includes:
    - success on a valid `scf.for`-based IR
    - success on a valid `scf.while`-based IR
    - syntax errors
    - missing contract targets
    - missing loop labels
    - AST emission
    - visibility of `poly.param` symbols
    - visibility of `poly.expr` symbols
- The old direct Python `lit` workflow is not used.
  - `cargo test` and `nix build` run the lit-style tests through Rust.

### Logistics Context

This milestone is 5 weeks long and is due on May 17, 2026. After this milestone, Milestone 2 begins (the creation of the `verif` MLIR dialect, which is what this language will lower to), which lasts for 3 weeks. This timeline should ideally give us time to finish the spec language design early so we can work on the `verif` dialect for spec language → `verif` dialect translation

## Syntax

### Overview

Some plain language explanations for the various keywords/constructs in the initial version of the language.

- `contract for <symbol> {/* region */}`: specifies the specification for the given circuit symbol `<symbol>`
    - Can be used for function and struct symbols
- `predicate <symbol>(inputs) {/* region */}`:
    - yields a single boolean expression as the predicate value via `return`
    - May be defined within a contract or outside of a contract
- `predicate <symbol>(inputs) = <expression>`
- `compute|witness`: specifies that the following statement applies only to witness computation.
    - Can apply to a single statement or a region
        - `compute ensure ...` or `compute { /* region */}`
- `constrain`: specifies that the following statement applies only to constraint generation.
    - Same as with `compute|witness`.
    - Note: unqualified statements will apply to both compute and constrain logic
- `require <expression>;`: pre-condition of the contract
- `ensure <expression>;`: post-condition of the contract
- `invariant for <loop attribute label> (<symbol>) {/* region */}` : loop invariant for a labeled loop with induction variable `<symbol>`
    - loop label attribute is embedded in the LLZK IR (e.g., `loop_label`).
    - For `scf.for` loops, the induction variable is found trivially
    - For `scf.while` loops (which are emitted by circom), the induction variable should be explicitly labeled (e.g., `induction_arg = "<number>"`), where `<number>` is the argument number of the loop body block.
- `let <symbol> = <expression>;`: bind the expression to the given spec-local symbol
- `let <symbol> = nondet;`: makes an unbounded felt
- `forall <symbol> in <range or array expression>, <expression>`: universal quantifier.
    - Is an expression
- `exists <symbol> in <range or array expression>, <expression>`: existential quantifier
- `unused <symbol>;`: explicit notation that `<symbol>` is intentionally unused in the given context.
- `len(<array symbol>)`: built-in length function

#### Notes on Types

- All scalar expressions are implicitly field elements
    - We may want to support other scalar types, or perhaps these should just be predicates as well (i.e., `int64` type vs just asserting `is_int64`)

### Parsing Expression Grammar (PEG)


- **`llzk_spec.pest`**: see [./src/grammar/llzk_spec.pest]


### Unsupported/Future Features

Here’s a list of some possible features that we may want for a future version of the spec language, but that we’re intentionally omitting in the first version for sake of time.

- Non-function/struct specific contracts (i.e., `contract Foo`)
    - Could be useful for specifying interactions between multiple circuit components, but then requires us to support generalized struct construction.
- Free functions
- Arbitrary loops
- Advanced typing

### Examples

#### Example 1: Circom `IsZero`

```
/* is_zero.circom */

template IsZero() {
  signal input in;
  signal output {binary} out;

  signal inv;

  inv <-- in!=0 ? 1/in : 0;

  out <== -in*inv +1;
  in*out === 0;
}

/* is_zero.spec */

contract for IsZero {
	ensure out == 0 || out == 1;
}

contract for IsZero {
	ensure in == 0 ? out == 1 : out == 0;
}
```

#### Example 2b: Circom `Num2Bits` with loop invariants

```
/* bitify.circom */

template Num2Bits(n) {
  signal input {maxbit} in;
  signal output out[n];
  in.maxbit = n;
  var lc1=0;

  var e2=1;
  for (var i = 0; i<n; i++) {
    out[i] <-- (in >> i) & 1;
    out[i] * (out[i] -1 ) === 0;
    lc1 += out[i] * e2;
    e2 = e2+e2;
  }

  lc1 === in;
}

/* bitify.llzk */

poly.template @Num2Bits {
    poly.param @n
    struct.def @Num2Bits {
      struct.member @out : !array.type<@n x !felt.type<"bn128">> {llzk.pub}
      function.def @compute(%arg0: !felt.type<"bn128">) -> !struct.type<@Num2Bits::@Num2Bits<[@n]>> attributes {function.allow_non_native_field_ops, function.allow_witness} {
        %self = struct.new : <@Num2Bits::@Num2Bits<[@n]>>
        %0 = poly.read_const @n : !felt.type<"bn128">
        %nondet = llzk.nondet : !array.type<@n x !felt.type<"bn128">>
        %felt_const_0 = felt.const  0 : <"bn128">
        %felt_const_1 = felt.const  1 : <"bn128">
        %felt_const_0_0 = felt.const  0 : <"bn128">
        %1:3 = scf.while (%arg1 = %felt_const_0, %arg2 = %felt_const_1, %arg3 = %felt_const_0_0) : (!felt.type<"bn128">, !felt.type<"bn128">, !felt.type<"bn128">) -> (!felt.type<"bn128">, !felt.type<"bn128">, !felt.type<"bn128">) **attributes {loop_label = "loop1", induction_arg = "2"}** {
          %2 = bool.cmp lt(%arg3, %0) : !felt.type<"bn128">, !felt.type<"bn128">
          scf.condition(%2) %arg1, %arg2, %arg3 : !felt.type<"bn128">, !felt.type<"bn128">, !felt.type<"bn128">
        } do {
        ^bb0(%arg1: !felt.type<"bn128">, %arg2: !felt.type<"bn128">, %arg3: !felt.type<"bn128">):
          %2 = felt.shr %arg0, %arg3 : !felt.type<"bn128">, !felt.type<"bn128">
          %felt_const_1_1 = felt.const  1 : <"bn128">
          %3 = felt.bit_and %2, %felt_const_1_1 : !felt.type<"bn128">, !felt.type<"bn128">
          %4 = cast.toindex %arg3 : !felt.type<"bn128">
          array.write %nondet[%4] = %3 : <@n x !felt.type<"bn128">>, !felt.type<"bn128">
          %5 = cast.toindex %arg3 : !felt.type<"bn128">
          %6 = array.read %nondet[%5] : <@n x !felt.type<"bn128">>, !felt.type<"bn128">
          %7 = felt.mul %6, %felt_const_1 : !felt.type<"bn128">, !felt.type<"bn128">
          %8 = felt.add %arg1, %7 : !felt.type<"bn128">, !felt.type<"bn128">
          %9 = felt.add %arg2, %arg2 : !felt.type<"bn128">, !felt.type<"bn128">
          %felt_const_1_2 = felt.const  1 : <"bn128">
          %10 = felt.add %arg3, %felt_const_1_2 : !felt.type<"bn128">, !felt.type<"bn128">
          scf.yield %8, %9, %10 : !felt.type<"bn128">, !felt.type<"bn128">, !felt.type<"bn128">
        }
        struct.writem %self[@out] = %nondet : <@Num2Bits::@Num2Bits<[@n]>>, !array.type<@n x !felt.type<"bn128">>
        function.return %self : !struct.type<@Num2Bits::@Num2Bits<[@n]>>
      }
      function.def @constrain(%arg0: !struct.type<@Num2Bits::@Num2Bits<[@n]>>, %arg1: !felt.type<"bn128">) attributes {function.allow_constraint, function.allow_non_native_field_ops} {
        %0 = poly.read_const @n : !felt.type<"bn128">
        %1 = struct.readm %arg0[@out] : <@Num2Bits::@Num2Bits<[@n]>>, !array.type<@n x !felt.type<"bn128">>
        %felt_const_0 = felt.const  0 : <"bn128">
        %felt_const_1 = felt.const  1 : <"bn128">
        %felt_const_0_0 = felt.const  0 : <"bn128">
        %2:3 = scf.while (%arg2 = %felt_const_1, %arg3 = %felt_const_0, %arg4 = %felt_const_0_0) : (!felt.type<"bn128">, !felt.type<"bn128">, !felt.type<"bn128">) -> (!felt.type<"bn128">, !felt.type<"bn128">, !felt.type<"bn128">) **attributes {loop_label = "loop1", induction_arg = "2"}** {
          %3 = bool.cmp lt(%arg4, %0) : !felt.type<"bn128">, !felt.type<"bn128">
          scf.condition(%3) %arg2, %arg3, %arg4 : !felt.type<"bn128">, !felt.type<"bn128">, !felt.type<"bn128">
        } do {
        ^bb0(%arg2: !felt.type<"bn128">, %arg3: !felt.type<"bn128">, %arg4: !felt.type<"bn128">):
          %3 = cast.toindex %arg4 : !felt.type<"bn128">
          %4 = array.read %1[%3] : <@n x !felt.type<"bn128">>, !felt.type<"bn128">
          %5 = cast.toindex %arg4 : !felt.type<"bn128">
          %6 = array.read %1[%5] : <@n x !felt.type<"bn128">>, !felt.type<"bn128">
          %felt_const_1_1 = felt.const  1 : <"bn128">
          %7 = felt.sub %6, %felt_const_1_1 : !felt.type<"bn128">, !felt.type<"bn128">
          %8 = felt.mul %4, %7 : !felt.type<"bn128">, !felt.type<"bn128">
          %felt_const_0_2 = felt.const  0 : <"bn128">
          constrain.eq %8, %felt_const_0_2 : !felt.type<"bn128">, !felt.type<"bn128">
          %9 = cast.toindex %arg4 : !felt.type<"bn128">
          %10 = array.read %1[%9] : <@n x !felt.type<"bn128">>, !felt.type<"bn128">
          %11 = felt.mul %10, %felt_const_1 : !felt.type<"bn128">, !felt.type<"bn128">
          %12 = felt.add %arg3, %11 : !felt.type<"bn128">, !felt.type<"bn128">
          %13 = felt.add %arg2, %arg2 : !felt.type<"bn128">, !felt.type<"bn128">
          %felt_const_1_3 = felt.const  1 : <"bn128">
          %14 = felt.add %arg4, %felt_const_1_3 : !felt.type<"bn128">, !felt.type<"bn128">
          scf.yield %13, %12, %14 : !felt.type<"bn128">, !felt.type<"bn128">, !felt.type<"bn128">
        }
        constrain.eq %2#1, %arg1 : !felt.type<"bn128">, !felt.type<"bn128">
        function.return
      }
    }
  }

/* bitify.spec */

contract for Num2Bits {
	invariant for loop1(i) {
		ensure out[i] == 0 || out[i] == 1;
		ensure in & (2 ** i) == out[i] * (2 ** i);
	}
}

```

#### Example 2b: Circom `Num2Bits` with quantifiers

```
/* bitify.circom */

template Num2Bits(n) {
  signal input {maxbit} in;
  signal output out[n];
  in.maxbit = n;
  var lc1=0;

  var e2=1;
  /* spec:label:loop1 */
  for (var i = 0; i<n; i++) {
    out[i] <-- (in >> i) & 1;
    out[i] * (out[i] -1 ) === 0;
    lc1 += out[i] * e2;
    e2 = e2+e2;
  }

  lc1 === in;
}

/* bitify.spec */

predicate bit_i_equals_out_i(in, out, i) {
  let bit_i = (in & 2**i) != 0 ? 1 : 0;
  return bit_i == out[i];
}

contract for Num2Bits {
	ensure forall o in out, o == 0 || o == 1;
	ensure forall i in 0..n, bit_i_equals_out_i(in, out, i);
}

```

#### Example 3: Circom `LessThan`

```
/* comparators.circom */

template LessThan(n) {
  assert(n <= 252);
  signal input in[2];
  signal output out;

	// See Num2Bits definition in Example 2.
  component n2b = Num2Bits(n+1);

  n2b.in <== in[0]+ (1<<n) - in[1];

  out <== 1-n2b.out[n];
}

/* comparators.spec */

contract for LessThan {
	require n <= 252;
	ensure out == 1 ? in[0] < in[1] : in[0] >= in[1];
}
```

#### Example 4: Zirgen Spec

```
/* one_hot.zir */

component OneHot<N: Val>(v: Val) {
  // Make N bit registers, with bit v set and all others 0
  public bits := for i : 0..N { NondetBitReg(Isz(i - v)) };
  // Verify exactly one bit is set
  reduce bits init 0 with Add = 1;
  // Verify the right bit is set
  reduce for i : 0..N { bits[i] * i } init 0 with Add = v;
  bits
}

/* one_hot.spec */
// alternative 1
contract for OneHot {
  // Bit `v` is within the array size, N
  ensure len(bits) == N && v <= N;
  // Bit `v` is 1, other bits are 0
	ensure forall i in 0..N, i == v ? bits[i] == 1 : bits[i] == 0;
}

// alternative 2
contract for OneHot {
  // Bit `v` is within the array size, N
  ensure len(bits) == N && v <= N
  // All elements are bits
	ensure forall b in bits, b == 0 || b == 1;
	// Some element is 1
	ensure exists b in bits, b == 1;
	// Only one element is one, because all pairs multiply to zero
	ensure forall i in 0..(N-1), forall j in (i+1)..N, (bits[i] * bits[j]) == 0;
	// Bit `v` is 1
	ensure bits[v] == 1;
}

// alternative 3
predicate all_bits_boolean(bit_arr) {
	return forall i in 0..len(bit_arr), bit_arr[i] == 0 || bit_arr[i] == 1;
}
contract for OneHot {
  // Bit `v` is within the array size, N
  ensure len(bits) == N && v <= N
  // All elements are bits
	ensure all_bits_boolean(bits);
	// Some element is 1
	ensure exists b in bits, b == 1;
	// Only one element is one, because all pairs multiply to zero
	ensure forall i in 0..(N-1), forall j in (i+1)..N, (bits[i] * bits[j]) == 0;
	// Bit `v` is 1
	ensure bits[v] == 1;
}
```

## Implementation Notes

- We should implement the spec language frontend in **Rust** rather than C++
    - Most users of LLZK seem to be Rust based, so we will likely cater more to future external contributors if we implement in Rust
- We can use this PEG parser (https://github.com/pest-parser/pest) as our workhorse for the spec language grammar
    - Documentation: https://pest.rs/book/

# Prompt context

The initial setup prompt is here: [./PROMPT.md]
