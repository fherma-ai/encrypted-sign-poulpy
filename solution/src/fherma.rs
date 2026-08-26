// GENERATED from sign/sign@1.0.0. Do not edit — `--update` rewrites it.
//
// The types your answer is written against, derived from the signature: one
// field per value parameter, per argument, per result. A tensor is typed,
// because the signature already settled what its elements are.

#[derive(Debug, Clone, Default)]
pub struct Tensor<T> {
    pub shape: Vec<i64>,
    pub data: Vec<T>,          // row-major
}

impl<T> Tensor<T> {
    pub fn count(&self) -> usize {
        self.shape.iter().product::<i64>() as usize
    }
}

#[derive(Debug, Clone, Default)]
pub struct Point {
    pub N: u64,   // uint
}

#[derive(Debug, Clone, Default)]
pub struct Inputs {
    pub xs: Tensor<f64>,   // tensor<N x f64>
}

#[derive(Debug, Clone, Default)]
pub struct Outputs {
    pub s: Tensor<f64>,   // tensor<N x f64>
}
