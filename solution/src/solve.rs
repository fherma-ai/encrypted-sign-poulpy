//! The sign of every element, over Poulpy's CKKS.
//!
//! One Chebyshev polynomial of degree 511, interpolating the ramp
//! clamp(x / 0.02) — the specification promises every examined value stays
//! 0.02 away from zero, so the ramp and the sign agree everywhere a verdict
//! is read. The library's Baby-Step Giant-Step evaluator spends nine
//! rescales of the budget; the power basis and the coefficients are public
//! material, built once in init.
use poulpy_ckks::api::CKKSPolynomialEvaluationOps;
use poulpy_ckks::layouts::CKKSPlaintextOwned;
use poulpy_ckks::polynomial::{BSGSPolynomial, Basis, EncodeBSGS, Polynomial};
use poulpy_ckks::power_basis::{PowerBasis, PowerBasisGen};
use poulpy_ckks::test_suite::helpers::{alloc_ct, upload_pt};
use poulpy_ckks::CoeffsMeta;
use poulpy_core::layouts::Base2K;
use poulpy_hal::api::ScratchOwnedBorrow;

use crate::envelope::{Backend, Ct, Env, Pt, PtOut};
use crate::fherma::{Inputs, Outputs, Point, Tensor};

const DEGREE: usize = 511;
const MARGIN: f64 = 0.02;

pub struct State {
    poly: BSGSPolynomial<CKKSPlaintextOwned<Backend>>,
}

/// The polynomial is public material: interpolated, encoded for BSGS and
/// uploaded once per point, never measured.
pub fn init(_p: &Point, env: &mut Env) -> State {
    let ramp = |x: f64| (x / MARGIN).clamp(-1.0, 1.0);
    let poly = Polynomial::chebyshev_interpolate(DEGREE, -1.0f64, 1.0, ramp)
        .expect("chebyshev interpolation");
    let meta = CoeffsMeta::from_delta_budget(env.params.prec_meta.log_delta, 8);
    let host = poly
        .encode_bsgs(&env.host_module, Base2K(env.params.base2k as u32), meta)
        .expect("encode bsgs");
    State { poly: host.map_baby_steps_ref(|pt| upload_pt(&env.module, pt)) }
}

pub fn encoding(env: &mut Env, inp: &Inputs) -> Vec<Pt> {
    vec![env.encode_reals(&inp.xs.data)]
}

pub fn run(state: &mut State, env: &mut Env, cts: Vec<Ct>) -> Vec<Ct> {
    let x = cts.into_iter().next().expect("one ciphertext");
    let mut basis = PowerBasis::new(Basis::Chebyshev, x);
    basis
        .populate(
            DEGREE,
            state.poly.log_split(),
            state.poly.parity(),
            &env.module,
            &env.tensor_key,
            &mut env.scratch.borrow(),
        )
        .expect("power basis");
    let mut out = alloc_ct(&env.params, &env.module, env.params.k);
    env.module
        .ckks_eval_poly_real_const_coeffs_from_power_basis(
            &mut out,
            &state.poly,
            &basis,
            &env.tensor_key,
            &mut env.scratch.borrow(),
        )
        .expect("evaluate");
    vec![out]
}

pub fn decoding(p: &Point, env: &mut Env, pts: Vec<PtOut>) -> Outputs {
    let values = env.decode_reals(&pts[0], p.N as usize);
    Outputs { s: Tensor { shape: vec![p.N as i64], data: values } }
}

pub fn free(state: State) {
    drop(state);
}
