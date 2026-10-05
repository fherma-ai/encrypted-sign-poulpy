// GENERATED for sign/sign@1.0.0. Do not edit — `--update` rewrites it.
//
// The envelope: everything cryptographic that is not the answer. It reads
// config.jsonc, builds the module, generates the keys, and encrypts and
// decrypts at the boundary. The secret key lives in this file and is not
// reachable from solve.rs: the four functions receive `Env`, and `Env` does
// not carry it.
//
// The security floor is enforced here, not trusted to the author: a ring
// that cannot carry the declared torus width at 128 bits classic refuses to
// seal, before anything is measured.
use std::collections::HashMap;
use std::fs;

use poulpy_ckks::api::{CKKSPlaintextVecOps, CKKSEncryptOps, CKKSDecryptOps};
use poulpy_ckks::layouts::{CKKSCiphertextOwned, CKKSModuleAlloc, CKKSPlaintextOwned};
use poulpy_ckks::test_suite::helpers::{
    alloc_ct, alloc_scratch, ckks_decrypt_with_prec, gen_atk, gen_sk_with_raw, gen_tsk, upload_pt,
};
use poulpy_ckks::test_suite::reference_encoder::ReferenceEncoder;
use poulpy_ckks::test_suite::CKKSTestParams;
use poulpy_ckks::{CKKSInfos, CKKSLayout, CKKSMeta, SetCKKSInfos, SlotsKind};
use poulpy_core::EncryptionLayout;
use poulpy_core::layouts::{
    Base2K, Degree, GLWEAutomorphismKeyPrepared, GLWELayout,
    GLWESecretPrepared, GLWETensorKeyPrepared, LWEInfos, Rank, TorusPrecision,
};
use poulpy_hal::api::ScratchOwnedBorrow;
use poulpy_hal::layouts::{GaloisElement as _, HostBytesBackend, Module, ScratchOwned};
use poulpy_hal::source::Source;

#[cfg(target_arch = "aarch64")]
pub use poulpy_cpu_arm::FFT64Neon as Backend;
#[cfg(target_arch = "x86_64")]
pub use poulpy_cpu_avx::FFT64Avx as Backend;

pub type Encoder = ReferenceEncoder<poulpy_cpu_ref::FFT64ReimTable<f64>>;
/// A packing on its way in: what encoding produces and encryption consumes.
pub type Pt = CKKSPlaintextOwned<Backend>;
/// A packing on its way out: what decryption produces and decoding reads.
pub type PtOut = CKKSPlaintextOwned<HostBytesBackend>;
pub type Ct = CKKSCiphertextOwned<Backend>;

/// The HE standard's table: the widest modulus a ring degree carries at
/// 128 bits classic. The envelope's floor.
const MAX_TORUS_BITS_128_CLASSIC: &[(usize, usize)] = &[
    (2048, 54),
    (4096, 109),
    (8192, 218),
    (16384, 438),
    (32768, 881),
    (65536, 1782),
];

pub struct Config {
    pub scheme: String,
    pub ring_degree: usize,
    pub base2k: usize,
    pub log_delta: usize,
    pub log_budget: usize,
    pub secret_hamming_weight: usize,
    pub dsize: usize,
    pub rotation_indexes: Vec<i64>,
    pub security: String,
}

impl Config {
    pub fn load(path: &str) -> Config {
        let raw = fs::read_to_string(path).expect("config.jsonc is unreadable");
        // JSONC: `//` to end of line is a comment.
        let stripped: String = raw
            .lines()
            .map(|line| line.split("//").next().unwrap_or(""))
            .collect::<Vec<_>>()
            .join("\n");
        let json: serde_json::Value =
            serde_json::from_str(&stripped).expect("config.jsonc is not valid JSONC");

        let number = |name: &str, fallback: u64| -> usize {
            json.get(name).and_then(|v| v.as_u64()).unwrap_or(fallback) as usize
        };
        let word = |name: &str, fallback: &str| -> String {
            json.get(name)
                .and_then(|v| v.as_str())
                .unwrap_or(fallback)
                .to_string()
        };

        Config {
            scheme: word("scheme", "ckks"),
            ring_degree: number("ring_degree", 8192),
            base2k: number("base2k", 19),
            log_delta: number("log_delta", 30),
            log_budget: number("log_budget", 122),
            secret_hamming_weight: number("secret_hamming_weight", 192),
            dsize: number("dsize", 1),
            rotation_indexes: json
                .get("rotation_indexes")
                .and_then(|v| v.as_array())
                .map(|list| list.iter().filter_map(|v| v.as_i64()).collect())
                .unwrap_or_default(),
            security: word("security", "128_classic"),
        }
    }
}

/// Everything the four functions may hold: the module, the parameters, the
/// evaluation keys the config declared, and the scratch every operation
/// borrows. The secret key is deliberately not here.
pub struct Env {
    pub params: CKKSTestParams,
    pub module: Module<Backend>,
    pub host_module: Module<HostBytesBackend>,
    pub encoder: Encoder,
    pub tensor_key: GLWETensorKeyPrepared<Vec<u8>, Backend>,
    /// Keyed by galois element: pass the whole map to `ckks_rotate_into`.
    pub rotation_keys: HashMap<i64, GLWEAutomorphismKeyPrepared<Vec<u8>, Backend>>,
    pub scratch: ScratchOwned<Backend>,
}

/// The envelope proper: the Env the answer sees, and the secret it does not.
pub struct Envelope {
    pub env: Env,
    sk: GLWESecretPrepared<Vec<u8>, Backend>,
}

impl Envelope {
    pub fn seal(cfg: &Config) -> Envelope {
        assert_eq!(cfg.scheme, "ckks", "this envelope speaks ckks and nothing else");
        assert_eq!(cfg.security, "128_classic", "unknown security floor");
        let k = cfg.log_delta + cfg.log_budget;
        let ceiling = MAX_TORUS_BITS_128_CLASSIC
            .iter()
            .find(|(n, _)| *n == cfg.ring_degree)
            .map(|(_, bits)| *bits)
            .unwrap_or_else(|| panic!("ring_degree {} is not a standard degree", cfg.ring_degree));
        assert!(
            k <= ceiling,
            "torus width {} bits needs a larger ring: degree {} carries at most {} at 128 bits classic",
            k, cfg.ring_degree, ceiling,
        );

        let params = CKKSTestParams {
            n: cfg.ring_degree,
            base2k: cfg.base2k,
            k,
            prec_meta: CKKSMeta {
                log_sparsity: 0,
                log_delta: cfg.log_delta,
                slots: SlotsKind::Complex,
            },
            prec_log_budget: cfg.log_budget.min(127usize.saturating_sub(cfg.log_delta)),
            hw: cfg.secret_hamming_weight,
            dsize: cfg.dsize,
            rank: 1,
        };

        let module = Module::<Backend>::new(params.n as u64);
        let host_module = Module::<HostBytesBackend>::new(params.n as u64);
        let encoder = Encoder::new(params.n / 2).expect("encoder");
        let (sk_raw, sk) = gen_sk_with_raw(&params, &module, &host_module, entropy());
        let mut scratch = alloc_scratch(&params, &module);
        let tensor_key = gen_tsk(&params, &module, &sk_raw, &mut scratch.borrow());
        let mut rotation_keys = HashMap::new();
        for &shift in &cfg.rotation_indexes {
            let galois = module.galois_element(shift);
            rotation_keys.insert(
                galois,
                gen_atk(&params, &module, galois, &sk_raw, &mut scratch.borrow()),
            );
        }
        // sk_raw drops here: only the prepared secret survives, inside the
        // envelope, and the evaluation keys are already cut.
        Envelope {
            env: Env { params, module, host_module, encoder, tensor_key, rotation_keys, scratch },
            sk,
        }
    }

    pub fn encrypt(&mut self, packings: Vec<Pt>) -> Vec<Ct> {
        let env = &mut self.env;
        let layout = GLWELayout {
            n: Degree(env.params.n as u32),
            base2k: Base2K(env.params.base2k as u32),
            k: TorusPrecision(env.params.k as u32),
            rank: Rank(1),
        };
        let enc = EncryptionLayout::new_from_default_sigma(layout).expect("encryption layout");
        packings
            .into_iter()
            .map(|pt| {
                let mut ct = alloc_ct(&env.params, &env.module, env.params.k);
                let mut xa = Source::new(entropy());
                let mut xe = Source::new(entropy());
                env.module
                    .ckks_encrypt_sk(&mut ct, &pt, &self.sk, &enc, &mut xe, &mut xa, &mut env.scratch.borrow())
                    .expect("encrypt");
                ct
            })
            .collect()
    }

    pub fn decrypt(&mut self, cts: Vec<Ct>) -> Vec<PtOut> {
        let env = &mut self.env;
        cts.iter()
            .map(|ct| {
                // Cap the headroom so the centered plaintext fits the decode
                // codec: dropping unused high-order budget is lossless.
                let log_delta = ct.log_delta();
                let log_budget = ct
                    .log_budget()
                    .min(env.params.prec().log_budget())
                    .min(127usize.saturating_sub(log_delta));
                let prec = CKKSLayout {
                    glwe_layout: GLWELayout {
                        n: ct.n(),
                        base2k: ct.base2k(),
                        k: TorusPrecision((log_delta + log_budget) as u32),
                        rank: Rank(1),
                    },
                    meta: CKKSMeta { log_sparsity: 0, log_delta, slots: SlotsKind::Complex },
                };
                ckks_decrypt_with_prec(&env.module, ct, &self.sk, prec, &mut env.scratch.borrow())
                    .expect("decrypt")
            })
            .collect()
    }
}

impl Env {
    /// The slot count: how many values one packing carries.
    pub fn slots(&self) -> usize {
        self.params.n / 2
    }

    /// Real values into one packing, zero-padded to the slot count. The
    /// imaginary lane is left empty; a layout that wants both writes its own
    /// encode against `self.encoder`.
    pub fn encode_reals(&mut self, values: &[f64]) -> Pt {
        let m = self.slots();
        assert!(values.len() <= m, "{} values into {} slots", values.len(), m);
        let mut re = values.to_vec();
        re.resize(m, 0.0);
        let im = vec![0.0f64; m];
        let prec = self.params.prec();
        let mut host_pt = self
            .host_module
            .ckks_pt_vec_alloc(Base2K(self.params.base2k as u32), prec.k());
        host_pt.set_meta(prec.meta());
        self.encoder.encode_reim(&mut host_pt, &re, &im).expect("encode");
        upload_pt(&self.module, &host_pt)
    }

    /// The real lane of a decrypted packing, first `count` slots.
    pub fn decode_reals(&mut self, pt: &PtOut, count: usize) -> Vec<f64> {
        let m = self.slots();
        let mut re = vec![0.0f64; m];
        let mut im = vec![0.0f64; m];
        self.encoder.decode_reim(pt, &mut re, &mut im).expect("decode");
        re.truncate(count);
        re
    }
}

/// Seed material from the operating system, exactly 32 bytes a call —
/// /dev/urandom is a stream without an end, so it is read, never slurped.
fn entropy() -> [u8; 32] {
    use std::io::Read;

    let mut seed = [0u8; 32];
    let filled = fs::File::open("/dev/urandom")
        .and_then(|mut source| source.read_exact(&mut seed))
        .is_ok();
    if !filled {
        // A container without /dev/urandom is broken, but a weak seed is
        // still better than a fixed one.
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        seed[..16].copy_from_slice(&now.as_nanos().to_le_bytes());
    }
    seed
}
