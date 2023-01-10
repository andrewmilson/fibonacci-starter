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
    Air, Matrix, ProofOptions, Prover, Trace, TraceInfo,
};
use pollster::block_on;
use std::time::Instant;

const NUM_FRI_QUERIES: usize = 30;
const LDE_BLOWUP_FACTOR: usize = 30;

fn main() {
    // project goal - convince a verifier we know the 65536th (2^16) fibonacci number
    let n = 2usize.pow(16);

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

    // 1. generate a nx1 matrix full of fibbonacci numbers (prover only)
    let fib_matrix = build_fib_matrix(n);

    // 2. generate STARK proof
    let prover = FibProver::new(proof_options);
    let trace = FibTrace::new(fib_matrix);
    println!("Generating proof");
    let now = Instant::now();
    let proof = pollster::block_on(prover.generate_proof(trace)).unwrap();
    println!("Generated proof in {:?}", now.elapsed());
    println!("Proof security {}-bit", proof.conjectured_security_level());

    // 3. verify STARK proof
    println!("Verifying proof");
    let now = Instant::now();
    proof.verify().unwrap();
    println!("Proof verified in {:?}", now.elapsed());
}

struct FibTrace {
    execution_trace: Matrix<Fp>,
}

impl FibTrace {
    fn new(execution_trace: Matrix<Fp>) -> Self {
        FibTrace { execution_trace }
    }

    fn last_fib_number(&self) -> Fp {
        let n = self.execution_trace.num_rows();
        self.execution_trace.get_row(n - 1).unwrap()[0]
    }
}

impl Trace for FibTrace {
    const NUM_BASE_COLUMNS: usize = 1;
    type Fp = Fp;
    type Fq = Fp;

    fn base_columns(&self) -> &Matrix<Self::Fp> {
        &self.execution_trace
    }
}

struct FibAir {
    info: ministark::TraceInfo,
    input: Fp,
    options: ProofOptions,
}

impl Air for FibAir {
    type Fp = Fp;
    type Fq = Fp;
    type PublicInputs = Fp;

    fn new(info: TraceInfo, input: Self::PublicInputs, options: ProofOptions) -> Self {
        FibAir {
            info,
            input,
            options,
        }
    }

    fn pub_inputs(&self) -> &Self::PublicInputs {
        &self.input
    }

    fn trace_info(&self) -> &TraceInfo {
        &self.info
    }

    fn options(&self) -> &ProofOptions {
        &self.options
    }

    fn constraints(&self) -> Vec<AlgebraicExpression<Self::Fp, Self::Fq>> {
        use AlgebraicExpression::*;

        let one = Constant(FieldConstant::Fp(Fp::one()));
        let claimed_nth_fib_num: AlgebraicExpression<Fp> = Constant(FieldConstant::Fp(self.input));

        // Domain we use to interpolate execution trace
        let trace_xs = self.trace_domain();
        let n = trace_xs.size();

        // NOTE: x^n - 1 = (x - ⍵_n^0)(x - ⍵_n^1)(x - ⍵_n^2)...(x - ⍵_n^(n-1))
        let vanish_all_rows: AlgebraicExpression<Fp> = X.pow(n) - &one;
        let vanish_first_row: AlgebraicExpression<Fp> = X - FieldConstant::Fp(trace_xs.element(0));
        let vanish_second_row: AlgebraicExpression<Fp> = X - FieldConstant::Fp(trace_xs.element(1));
        let vanish_last_row: AlgebraicExpression<Fp> =
            X - FieldConstant::Fp(trace_xs.element(n - 1));

        let column = 0;
        let row_offset = 0;
        let curr_row: AlgebraicExpression<Fp> = Trace(column, row_offset);

        let column = 0;
        let row_offset = -1;
        let before_row: AlgebraicExpression<Fp> = Trace(column, row_offset);

        let column = 0;
        let row_offset = -2;
        let two_before_row: AlgebraicExpression<Fp> = Trace(column, row_offset);

        vec![
            // 1. first row must equal 1
            // 2. second row must equal 1
            // 3. remainig rows must equal the sum of their two preceding rows
            // 4. last row must equal the the prover's claimed `n`th fibonacci number
            todo!(),
        ]
    }
}

struct FibProver {
    options: ProofOptions,
}

impl Prover for FibProver {
    type Fp = Fp;
    type Fq = Fp;
    type Air = FibAir;
    type Trace = FibTrace;

    fn new(options: ProofOptions) -> Self {
        FibProver { options }
    }

    fn get_pub_inputs(&self, trace: &Self::Trace) -> <Self::Air as Air>::PublicInputs {
        trace.last_fib_number()
    }

    fn options(&self) -> ProofOptions {
        self.options
    }
}

/// Fills a matrix with the fibonacci numbers
//  P(x)
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
