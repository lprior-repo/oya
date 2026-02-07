# Functional vs Imperative Rust Benchmark Results

This document presents comparative benchmarks between imperative and functional programming approaches in Rust, measured using Criterion 0.5.

## Environment

- **Rust Edition**: 2021
- **Crate**: oya-workflow
- **Benchmark Framework**: criterion 0.5
- **Date**: 2025-02-07

## Summary

The benchmarks reveal that functional and imperative approaches in Rust have **nearly identical performance** in most cases, with compiler optimizations erasing the theoretical overhead of functional abstractions. The key findings are:

1. **Iterator chains vs for-loops**: No significant difference (< 1%)
2. **Arc vs Clone**: 5-6x faster for shared data
3. **im::HashMap vs std::collections::HashMap**: Trade-offs between immutability and performance
4. **Error handling**: Negligible difference between `?` and `match`
5. **Allocation-heavy operations**: Sometimes favor functional approach

---

## Benchmark 1: Loop vs Iterator

### Simple Sum (1000 elements)

| Approach | Time | Throughput |
|----------|------|------------|
| **imperative_loop** | 93.75 ns | 10.67M ops/sec |
| **functional_iterator** | 93.83 ns | 10.66M ops/sec |

**Result**: Tie (0.09% difference - statistically insignificant)

The Rust compiler optimizes both approaches to nearly identical assembly. The iterator version is equally fast while being more expressive.

### Filter + Map (1000 elements)

| Approach | Time | Throughput |
|----------|------|------------|
| **imperative_filter** | 518.55 ns | 1.93M ops/sec |
| **functional_filter** | 469.50 ns | 2.13M ops/sec |

**Result**: Functional is **9.5% faster** ⚡

Surprisingly, the functional filter+map chain is faster. This is likely due to:
- Better loop optimizations in the iterator implementation
- Reduced bounds checking
- More predictable memory access patterns

### Nested Loops (100x100)

| Approach | Time | Throughput |
|----------|------|------------|
| **imperative_nested_loop** | 4.03 µs | 248K ops/sec |
| **functional_nested_iter** | 4.60 µs | 217K ops/sec |

**Result**: Imperative is **12.4% faster** 🏃

For deeply nested loops, imperative code has a slight edge due to:
- Fewer iterator object allocations
- Direct memory access without adapter overhead
- Better register utilization in tight loops

---

## Benchmark 2: Clone vs Arc

### Deep Clone (10KB vector)

| Approach | Time | Comparison |
|----------|------|------------|
| **deep_clone** | 43.73 ns | Baseline |
| **arc_clone** | 7.28 ns | **6.0x faster** ⚡⚡⚡ |

**Result**: Arc is **6x faster** for cloning shared data

This is the most dramatic difference in the suite. Arc::clone() only increments a reference count (atomic operation), while deep clone copies all 10KB.

### Read Performance (1KB vector)

| Approach | Time | Throughput |
|----------|------|------------|
| **owned_access** | 41.14 ns | 24.3M ops/sec |
| **arc_access** | 39.83 ns | 25.1M ops/sec |

**Result**: Arc is **3.2% faster** for reads

Arc has no performance penalty for reads. The compiler optimizes away the indirection.

### Struct Clone (complex struct)

| Approach | Time | Comparison |
|----------|------|------------|
| **struct_clone** | 2.11 µs | Baseline |
| **struct_arc_clone** | 7.39 ns | **285x faster** ⚡⚡⚡ |

**Result**: Arc is **285x faster** for complex structs

For structs with large nested data (Vec + metadata), Arc's advantage becomes massive.

---

## Benchmark 3: HashMap Operations

### Insert (100 items)

| Approach | Time | Comparison |
|----------|------|------------|
| **std_hashmap_insert** | 6.02 µs | Baseline |
| **im_hashmap_insert** | 24.85 µs | **4.1x slower** 🐌 |

**Result**: std::HashMap is **4x faster** for inserts

im::HashMap creates persistent data structures, requiring structural sharing on each update. This has significant overhead for mutation-heavy workloads.

### Lookup (100 items)

| Approach | Time | Throughput |
|----------|------|------------|
| **std_hashmap_lookup** | 385.24 ns | 2.60M ops/sec |
| **im_hashmap_lookup** | 357.83 ns | 2.79M ops/sec |

**Result**: im::HashMap is **7.1% faster** for reads ⚡

Despite persistent data structures, im::HashMap has slightly faster lookups due to:
- Better cache locality (HAMT structure)
- Optimized hash trie implementation

### Clone (100 items)

| Approach | Time | Comparison |
|----------|------|------------|
| **std_hashmap_clone** | 32.10 ns | Baseline |
| **im_hashmap_clone** | 14.98 ns | **2.1x faster** ⚡ |

**Result**: im::HashMap is **2.1x faster** to clone

This is the key advantage of persistent data structures. Cloning an im::HashMap is O(1) (structural sharing), while std::HashMap is O(n).

### Iteration (100 items)

| Approach | Time | Throughput |
|----------|------|------------|
| **std_hashmap_iterate** | 38.09 ns | 26.3M ops/sec |
| **im_hashmap_iterate** | 1.42 µs | 704K ops/sec |

**Result**: std::HashMap is **37x faster** for iteration 🐌

The HAMT structure in im::HashMap has higher traversal overhead during iteration.

### When to use im::HashMap:

- ✅ **Use when**: You need frequent snapshots/undos, functional updates, or concurrent reads
- ❌ **Avoid when**: Performance-critical mutation-heavy workloads, heavy iteration

---

## Benchmark 4: Error Handling

### Success Path (three chained operations)

| Approach | Time | Throughput |
|----------|------|------------|
| **functional_error_handling** (`?`) | 3.31 ns | 302M ops/sec |
| **imperative_error_handling** (match) | 3.26 ns | 307M ops/sec |

**Result**: Match is **1.5% faster** (negligible)

Both approaches compile to nearly identical code. The `?` operator has zero overhead.

### Error Path

| Approach | Time | Throughput |
|----------|------|------------|
| **functional_error_path** | 3.02 ns | 331M ops/sec |
| **imperative_error_path** | 0.78 ns | 1.28B ops/sec |

**Result**: Match is **3.9x faster** on error path 🐌

Early returns in match statements allow the compiler to eliminate more code on the error path.

### Chained Results (three operations)

| Approach | Time | Throughput |
|----------|------|------------|
| **functional_chain** (`and_then`) | 2.57 ns | 389M ops/sec |
| **imperative_chain** (match) | 2.53 ns | 395M ops/sec |

**Result**: Match is **1.6% faster** (negligible)

**Recommendation**: Use the `?` operator for readability. The performance difference is negligible in real applications.

---

## Key Takeaways

### 1. Zero-Cost Abstractions Work
Rust's promise of "zero-cost abstractions" holds true. Iterator chains, functional combinators, and Result operators have minimal overhead compared to imperative code.

### 2. Arc is Essential for Shared Data
For shared data structures, `Arc::clone()` is **5-300x faster** than deep cloning. Always prefer Arc when you need shared ownership.

### 3. Persistent Data Structures Trade Performance for Immutability
`im::HashMap` provides O(1) cloning and functional updates but is:
- **4x slower** for inserts
- **37x slower** for iteration
- **2x faster** for cloning

Use when you need frequent snapshots or undo functionality.

### 4. Prefer Functional Style for Readability
Given the minimal performance differences (< 5% in most cases), prefer the functional style for:
- ✅ Better composability
- ✅ Easier testing
- ✅ Fewer bugs (no mutable state)
- ✅ Clearer intent

### 5. Optimize Based on Measurements
- Nested loops: Imperative has edge
- Filter/map chains: Functional often faster
- Shared data: Arc is mandatory
- Error handling: Use `?` operator

---

## Performance Optimization Tips

1. **Profile before optimizing**: These microbenchmarks may not reflect real-world performance
2. **Use criterion**: Always benchmark with realistic data sizes
3. **Check assembly**: Use `cargo asm` to see what the compiler generates
4. **Prefer Arc over Clone**: For shared data > 1KB
5. **Consider im crates**: When you need persistent data structures
6. **Embrace functional style**: Readability matters more than 1-2% differences

---

## Running the Benchmarks

```bash
# Run all benchmarks
moon run :bench

# Run specific benchmark suite
cargo bench -p oya-workflow --bench loop_vs_iterator
cargo bench -p oya-workflow --bench clone_vs_arc
cargo bench -p oya-workflow --bench hashmap_vs_im
cargo bench -p oya-workflow --bench error_handling

# Generate HTML report
cargo bench -p oya-workflow -- --save-baseline main
```

Results are saved to `target/criterion/` with interactive HTML reports.

---

## Conclusion

**Rust enables functional programming without performance penalties**. The compiler's aggressive optimizations mean that choosing between imperative and functional styles should be based on:

1. **Code readability** (functional wins)
2. **Domain requirements** (immutability vs mutability)
3. **Team familiarity** (both are equally valid)

The performance differences are so small that other factors (maintainability, testability, correctness) should drive your architectural decisions.

---

**Generated by**: oya-workflow benchmark suite
**Criterion Version**: 0.5
**Commit**: Benchmark suite creation for functional-rust-generator skill
