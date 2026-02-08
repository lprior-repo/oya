#![allow(dead_code)]

use criterion::{black_box, criterion_group, criterion_main, Criterion};

#[derive(Debug)]
pub enum ComputationError {
    DivisionByZero,
}

// Functional approach with Result and ?
pub fn functional_divide(a: f64, b: f64) -> Result<f64, ComputationError> {
    if b == 0.0 {
        return Err(ComputationError::DivisionByZero);
    }
    Ok(a / b)
}

pub fn functional_computation(input: f64) -> Result<f64, ComputationError> {
    let step1 = functional_divide(input, 2.0)?;
    let step2 = functional_divide(step1, 3.0)?;
    let step3 = functional_divide(step2, 4.0)?;
    Ok(step3)
}

// Imperative match-based approach
pub fn imperative_computation(input: f64) -> Result<f64, ComputationError> {
    let step1 = functional_divide(input, 2.0)?;
    let step2 = functional_divide(step1, 3.0)?;
    let step3 = functional_divide(step2, 4.0)?;

    Ok(step3)
}

pub fn benchmark_error_propagation(c: &mut Criterion) {
    let input = 1000.0;

    c.bench_function("functional_error_handling", |b| {
        b.iter(|| black_box(functional_computation(input)))
    });

    c.bench_function("imperative_error_handling", |b| {
        b.iter(|| black_box(imperative_computation(input)))
    });
}

pub fn benchmark_error_path(c: &mut Criterion) {
    let input = 1000.0;

    c.bench_function("functional_error_path", |b| {
        b.iter(|| black_box(functional_divide(input, 0.0)))
    });

    c.bench_function("imperative_error_path", |b| {
        b.iter(|| functional_divide(input, 0.0))
    });
}

pub fn benchmark_chained_results(c: &mut Criterion) {
    fn add_one(x: i32) -> Result<i32, ComputationError> {
        Ok(x + 1)
    }

    fn multiply_two(x: i32) -> Result<i32, ComputationError> {
        Ok(x * 2)
    }

    c.bench_function("functional_chain", |b| {
        b.iter(|| {
            let result = Ok(10);
            let result = result.and_then(add_one);
            let result = result.and_then(multiply_two);
            let result = result.and_then(add_one);
            black_box(result)
        })
    });

    c.bench_function("imperative_chain", |b| {
        b.iter(|| {
            let mut result = Ok(10);
            result = match result {
                Ok(x) => add_one(x),
                Err(e) => Err(e),
            };
            result = match result {
                Ok(x) => multiply_two(x),
                Err(e) => Err(e),
            };
            result = match result {
                Ok(x) => add_one(x),
                Err(e) => Err(e),
            };
            black_box(result)
        })
    });
}

criterion_group!(
    benches,
    benchmark_error_propagation,
    benchmark_error_path,
    benchmark_chained_results
);
criterion_main!(benches);
