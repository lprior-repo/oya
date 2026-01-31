# Iterator & Result Combinators Reference

Complete reference of available combinators.

## Result Combinators

### Transform Value

| Method | Input | Output | Use |
|--------|-------|--------|-----|
| `map` | `Result<T>` | `Result<U>` | Transform success value |
| `map_err` | `Result<E>` | `Result<F>` | Transform error |
| `map_or` | `Result<T>` | `U` | Transform to type U or default |
| `map_or_else` | `Result<T>` | `U` | Transform or compute default |

```rust
Ok(5)
    .map(|x| x * 2)           // Ok(10)
    .map_err(|e| wrap_error(e))
    .map_or(0, |x| x + 1)     // 11
```

### Chain Operations

| Method | Use |
|--------|-----|
| `and_then` | Chain fallible operations |
| `or_else` | Alternative fallible operation |

```rust
Ok(5)
    .and_then(|x| {
        if x > 0 { Ok(x * 2) }
        else { Err(Error::Invalid) }
    })  // Ok(10)
    .or_else(|_| Ok(0))  // Ok(10) - not taken
```

### Extract Value

| Method | Returns | Use |
|--------|---------|-----|
| `unwrap_or` | `T` | Get value or default |
| `unwrap_or_else` | `T` | Get value or compute default |
| `ok` | `Option<T>` | Convert to Option |
| `err` | `Option<E>` | Extract error as Option |

```rust
Ok(5).unwrap_or(0)                    // 5
Err::<i32, _>(Error::X).unwrap_or(0)  // 0

Ok(5).unwrap_or_else(|_| 99)          // 5
Err::<i32, _>(Error::X)
    .unwrap_or_else(|_| 99)           // 99

Ok(5).ok()     // Some(5)
Err(Error::X).err()  // Some(Error::X)
```

### Inspect

| Method | Use |
|--------|-----|
| `inspect` | Inspect value, return self |
| `inspect_err` | Inspect error, return self |

```rust
Ok(5)
    .inspect(|x| println!("value: {}", x))
    .inspect_err(|e| eprintln!("error: {}", e))
    .map(|x| x * 2)
```

### Test

| Method | Returns | Use |
|--------|---------|-----|
| `is_ok` | `bool` | Check if Ok |
| `is_err` | `bool` | Check if Err |
| `contains` | `bool` | Check if Ok and equals value |
| `contains_err` | `bool` | Check if Err and equals error |

```rust
let result = Ok(5);
result.is_ok()         // true
result.is_err()        // false
result.contains(&5)    // true
```

## Option Combinators

### Transform

| Method | Use |
|--------|-----|
| `map` | Transform Some value |
| `map_or` | Transform or provide default |
| `map_or_else` | Transform or compute default |

```rust
Some(5)
    .map(|x| x * 2)              // Some(10)
    .map_or(0, |x| x + 1)        // 11
    .map_or_else(|| 99, |x| x)   // 11
```

### Chain

| Method | Use |
|--------|-----|
| `and_then` | Chain optional operations |
| `or` | Provide alternative Option |
| `or_else` | Compute alternative Option |

```rust
Some(5)
    .and_then(|x| {
        if x > 0 { Some(x * 2) }
        else { None }
    })  // Some(10)
    .or(Some(0))  // Some(10)
    .or_else(|| Some(99))  // Some(10)
```

### Extract

| Method | Returns | Use |
|--------|---------|-----|
| `unwrap_or` | `T` | Get value or default |
| `unwrap_or_else` | `T` | Get value or compute |
| `expect` | `T` | Get value or panic (FORBIDDEN) |

```rust
Some(5).unwrap_or(0)  // 5
None::<i32>.unwrap_or(0)  // 0

Some(5).unwrap_or_else(|| 99)  // 5
None::<i32>.unwrap_or_else(|| 99)  // 99
```

### Test

| Method | Returns | Use |
|--------|---------|-----|
| `is_some` | `bool` | Check if Some |
| `is_none` | `bool` | Check if None |
| `contains` | `bool` | Check if Some and equals |

```rust
Some(5).is_some()      // true
Some(5).is_none()      // false
Some(5).contains(&5)   // true
```

## Iterator Combinators

### Transform Each

| Method | Use |
|--------|-----|
| `map` | Apply function to each |
| `flat_map` | Map then flatten |
| `filter_map` | Filter + map combined |

```rust
vec![1, 2, 3]
    .iter()
    .map(|x| x * 2)        // [2, 4, 6]
    .flat_map(|x| vec![x, x])  // [2,2,4,4,6,6]
    .filter_map(|x| if x > 3 { Some(x) } else { None })  // [4,4,6,6]
```

### Filter

| Method | Use |
|--------|-----|
| `filter` | Keep matching |
| `take_while` | Take while predicate true |
| `skip_while` | Skip while predicate true |

```rust
vec![1, 2, 3, 4, 5]
    .iter()
    .filter(|x| x % 2 == 0)  // [2, 4]
    .take_while(|x| x < 4)   // [2, 4] (stops at 5)
    .collect()
```

### Accumulate

| Method | Returns | Use |
|--------|---------|-----|
| `fold` | `T` | Accumulate to single value |
| `try_fold` | `Result<T>` | Fold with error handling |
| `scan` | `Iterator` | Fold while iterating |

```rust
vec![1, 2, 3, 4, 5]
    .iter()
    .fold(0, |acc, x| acc + x)  // 15

vec![1, 2, 3]
    .iter()
    .try_fold(0, |acc, x| {
        if x > 2 { Err("too big") }
        else { Ok(acc + x) }
    })  // Err("too big")

vec![1, 2, 3]
    .iter()
    .scan(0, |acc, x| {
        *acc += x;
        Some(*acc)
    })  // [1, 3, 6]
```

### Partition

| Method | Returns | Use |
|--------|---------|-----|
| `partition` | `(Vec, Vec)` | Split into two groups |

```rust
let (evens, odds): (Vec<_>, Vec<_>) = (1..=5)
    .partition(|x| x % 2 == 0);
// evens = [2, 4]
// odds = [1, 3, 5]
```

### Group (with itertools)

```rust
use itertools::Itertools;

vec!["apple", "apricot", "banana"]
    .into_iter()
    .group_by(|s| s.chars().next().unwrap())
    .into_iter()
    .map(|(k, g)| (k, g.collect::<Vec<_>>()))
    // [('a', [...]), ('b', [...])]
```

### Zip & Combine

| Method | Use |
|--------|-----|
| `zip` | Combine two iterators |
| `chain` | Concatenate iterators |
| `cycle` | Repeat infinitely |

```rust
let a = vec![1, 2, 3];
let b = vec!['a', 'b', 'c'];

a.iter().zip(b.iter())
    // [(1, 'a'), (2, 'b'), (3, 'c')]

a.iter().chain(b.iter())
    // [1, 2, 3, 'a', 'b', 'c']

a.iter().cycle()  // 1, 2, 3, 1, 2, 3, ...
```

### Skip & Take

| Method | Use |
|--------|-----|
| `skip` | Skip n elements |
| `take` | Take first n |
| `skip_while` | Skip while predicate |
| `take_while` | Take while predicate |

```rust
vec![1, 2, 3, 4, 5]
    .iter()
    .skip(2)        // [3, 4, 5]
    .take(2)        // [3, 4]
```

### Test

| Method | Returns | Use |
|--------|---------|-----|
| `any` | `bool` | Any element matches |
| `all` | `bool` | All elements match |
| `find` | `Option` | First matching element |
| `position` | `Option` | Index of first match |

```rust
vec![1, 2, 3, 4, 5]
    .iter()
    .any(|x| x > 3)     // true
    .all(|x| x > 0)     // true
    .find(|x| x > 3)    // Some(4)
    .position(|x| x > 3)  // Some(3)
```

### Collect

| Method | Collects into |
|--------|---------------|
| `collect` | `Vec`, `HashMap`, `Result<Vec>`, etc |
| `collect::<()>` | Unit (side effects only) |
| `collect::<Result<T>>` | Error short-circuits |

```rust
vec![1, 2, 3]
    .iter()
    .map(|x| x * 2)
    .collect::<Vec<_>>()  // [2, 4, 6]

vec!["1", "2", "3"]
    .iter()
    .map(|s| s.parse::<i32>())
    .collect::<Result<Vec<_>>>()  // Ok([1, 2, 3]) or Err
```

## Chaining Combinators

```rust
vec![1, 2, 3, 4, 5]
    .iter()
    .filter(|x| x % 2 == 0)    // [2, 4]
    .map(|x| x * 2)            // [4, 8]
    .fold(0, |acc, x| acc + x) // 12
```

## Performance Tips

- Iterator chains are lazy (no intermediate allocations)
- Combinators compile to tight loops
- `collect()` is the only materialization point
- Use iterators for data pipelines

## The Philosophy

> "Iterator combinators are lazy, composable, and often faster than imperative loops."

Chain them expressively. Let the compiler optimize to efficient code.

---

**Next**: [Testing](07_TESTING.md)
