#![feature(allocator_api)]
use ark_ff::One;
use ark_poly::domain::EvaluationDomain;
use gpu_poly::{
    allocator::PageAlignedAllocator, fields::p18446744069414584321::Fp,
    fields::p18446744069414584321::Fq3,
};
use ministark::{
    constraints::{AlgebraicExpression, FieldConstant},
    hints::Hints,
    Air, Matrix, ProofOptions, Prover, Trace,
};
use pollster::block_on;
use std::time::Instant;

fn main() {
    // project goal
    // ============
    // convince a verifier we know the 65536th fibonacci number
    let n = 65536;
    let fib_matrix = build_fib_matrix(n);

    // proof options for 128 bit security
    let num_fri_queries = 30;
    let lde_blowup_factor = 16;
    let grinding_factor = 16;
    let fri_folding_factor = 8;
    let fri_max_remainder_size = 64;
    let proof_options = ProofOptions::new(
        num_fri_queries,
        lde_blowup_factor,
        grinding_factor,
        fri_folding_factor,
        fri_max_remainder_size,
    );

    // 1. generate STARK proof
    let now = Instant::now();
    // ...
    println!("Generated proof in {:?}", now.elapsed());

    // 2. verify STARK proof
    let now = Instant::now();
    // ...
    println!("Proof verified in {:?}", now.elapsed());
}

/// Fills a matrix with the fibonacci numbers
// ┌───────┐
// │ 1     │ <- P(⍵_n^0) = 1
// ├───────┤
// │ 1     │ <- P(⍵_n^1) = 1
// ├───────┤
// │ 2     │ <- P(⍵_n^2) = 2
// ├───────┤
// │ 3     │ <- ...
// ├───────┤
// │ 5     │
// ├───────┤
// │  ...  │
// ├───────┤
// │ fib_n │
// └───────┘
fn build_fib_matrix(n: usize) -> Matrix<Fp> {
    assert!(n.is_power_of_two());

    // The GPU only accepts page aligned memory
    let mut column = Vec::new_in(PageAlignedAllocator);

    // initial fibonacci numbers
    column.push(Fp::one());
    column.push(Fp::one());

    for i in 2..n {
        column.push(column[i - 1] + column[i - 2]);
    }

    Matrix::new(vec![column])
}
