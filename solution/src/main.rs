// GENERATED for sign/sign@1.0.0. Do not edit — `--update` rewrites it.
//
//     ./fherma-solution <point directory>
//
// Reads the directory a bundle prepared, answers every case in it, and writes
// what each stage cost. Only the call to solve::run is the score; keygen,
// encoding, encryption, decryption and decoding are timed apart and reported
// beside it.
mod envelope;
mod fherma;
mod solve;

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use envelope::{Config, Envelope};
use fherma::{Inputs, Outputs, Point, Tensor};

type Anyhow<T> = Result<T, Box<dyn std::error::Error>>;

/// Little-endian on the wire, and on every platform this runs on, so the copy
/// is the decode.
trait Wire: Sized + Copy + Default {
    fn from_wire(bytes: &[u8]) -> Self;
    fn to_wire(self) -> Vec<u8>;
}

macro_rules! wire {
    ($($t:ty),*) => {$(
        impl Wire for $t {
            fn from_wire(bytes: &[u8]) -> Self {
                Self::from_le_bytes(bytes.try_into().expect("width"))
            }
            fn to_wire(self) -> Vec<u8> { self.to_le_bytes().to_vec() }
        }
    )*};
}
wire!(i8, i16, i32, i64, u8, u16, u32, u64, f32, f64);

fn number(text: &str, key: &str) -> Anyhow<f64> {
    let quoted = format!("\"{key}\"");
    let at = text.find(&quoted).ok_or_else(|| format!("no {key} in manifest.json"))?;
    let colon = text[at..].find(':').ok_or("malformed manifest")? + at;
    let rest = text[colon + 1..].trim_start();
    let end = rest
        .find(|c: char| !c.is_ascii_digit() && c != '-' && c != '.' && c != 'e')
        .unwrap_or(rest.len());
    Ok(rest[..end].parse()?)
}

fn read_tensor<T: Wire>(dir: &Path, name: &str, shape: Vec<i64>) -> Anyhow<Tensor<T>> {
    let raw = fs::read(dir.join(format!("{name}.bin")))?;
    let width = std::mem::size_of::<T>();
    let count: usize = shape.iter().product::<i64>() as usize;
    if raw.len() != count * width {
        return Err(format!("{name}: {} bytes for {count} values", raw.len()).into());
    }
    Ok(Tensor { shape, data: raw.chunks_exact(width).map(T::from_wire).collect() })
}

fn read_one<T: Wire>(dir: &Path, name: &str) -> Anyhow<T> {
    let raw = fs::read(dir.join(format!("{name}.bin")))?;
    if raw.len() != std::mem::size_of::<T>() {
        return Err(format!("{name}: {} bytes for one value", raw.len()).into());
    }
    Ok(T::from_wire(&raw))
}

fn write_tensor<T: Wire>(dir: &Path, name: &str, tensor: &Tensor<T>) -> Anyhow<()> {
    let mut raw = Vec::with_capacity(tensor.data.len() * std::mem::size_of::<T>());
    for value in &tensor.data {
        raw.extend_from_slice(&value.to_wire());
    }
    fs::write(dir.join(format!("{name}.bin")), raw)?;
    Ok(())
}

fn write_one<T: Wire>(dir: &Path, name: &str, value: T) -> Anyhow<()> {
    fs::write(dir.join(format!("{name}.bin")), value.to_wire())?;
    Ok(())
}

/// One case's row in results.json, every stage said apart.
#[allow(clippy::too_many_arguments)]
fn ok_row(
    i: usize,
    seconds: f64,
    cts_in: usize,
    cts_out: usize,
    encoding_s: f64,
    encrypt_s: f64,
    decrypt_s: f64,
    decoding_s: f64,
) -> String {
    format!(
        concat!(
            "{{\"i\":{},\"seconds\":{:.9},\"status\":\"ok\"",
            ",\"ciphertexts_in\":{},\"ciphertexts_out\":{}",
            ",\"encoding_s\":{:.9},\"encrypt_s\":{:.9}",
            ",\"decrypt_s\":{:.9},\"decoding_s\":{:.9}}}"
        ),
        i, seconds, cts_in, cts_out, encoding_s, encrypt_s, decrypt_s, decoding_s
    )
}

fn crashed_row(i: usize, why: &str) -> String {
    let clean: String = why
        .chars()
        .take(200)
        .map(|c| if c == '"' || c == '\\' { ' ' } else if c == '\n' { ' ' } else { c })
        .collect();
    format!("{{\"i\":{i},\"seconds\":null,\"status\":\"crashed\",\"note\":\"{clean}\"}}")
}

/// Written after every case, not at the end: a process killed on its timeout
/// has still done the cases before it.
fn report(out: &Path, envelope_s: f64, init_s: f64, cases: &[String]) {
    let body = format!(
        "{{\"envelope_s\":{envelope_s:.9},\"init_s\":{init_s:.9},\"cases\":[{}]}}",
        cases.join(",")
    );
    let _ = fs::write(out.join("results.json"), body);
}

fn main() -> Anyhow<()> {
    let root = PathBuf::from(
        std::env::args().nth(1).ok_or("usage: fherma-solution <point directory>")?,
    );
    let manifest = fs::read_to_string(root.join("manifest.json"))?;

    let p = Point {
        N: number(&manifest, "N")? as _,
    };
    let total = number(&manifest, "cases")? as usize;

    let answers_root = root.join("out");
    fs::create_dir_all(&answers_root)?;

    // The config travels with the solution, not the point; the point's own
    // copy wins when a runner lays one there.
    let config_path = if root.join("config.jsonc").exists() {
        root.join("config.jsonc")
    } else {
        PathBuf::from("config.jsonc")
    };
    let sealing = Instant::now();
    let cfg = Config::load(config_path.to_str().ok_or("config path")?);
    let mut envelope = Envelope::seal(&cfg);
    let envelope_s = sealing.elapsed().as_secs_f64();

    let setup = Instant::now();
    let mut state = solve::init(&p, &mut envelope.env);
    let init_s = setup.elapsed().as_secs_f64();

    let mut cases: Vec<String> = Vec::new();
    report(&answers_root, envelope_s, init_s, &cases);

    for i in 0..total {
        let where_ = root.join("cases").join(format!("{i:06}"));
        let answers = answers_root.join(format!("{i:06}"));

        let inp = match read_case(&where_, &p) {
            Ok(inp) => inp,
            Err(failure) => {
                cases.push(crashed_row(i, &format!("reading the case: {failure}")));
                report(&answers_root, envelope_s, init_s, &cases);
                continue;
            }
        };

        // Every stage is timed, and only one is the score.
        let mark = Instant::now();
        let packings = solve::encoding(&mut envelope.env, &inp);
        let encoding_s = mark.elapsed().as_secs_f64();

        let mark = Instant::now();
        let cts = envelope.encrypt(packings);
        let encrypt_s = mark.elapsed().as_secs_f64();
        let cts_in = cts.len();

        // Monotonic, and around the call and nothing else.
        let started = Instant::now();
        let out_cts = solve::run(&mut state, &mut envelope.env, cts);
        let seconds = started.elapsed().as_secs_f64();
        let cts_out = out_cts.len();

        let mark = Instant::now();
        let pts = envelope.decrypt(out_cts);
        let decrypt_s = mark.elapsed().as_secs_f64();

        let mark = Instant::now();
        let answer = solve::decoding(&p, &mut envelope.env, pts);
        let decoding_s = mark.elapsed().as_secs_f64();

        if let Err(failure) = write_case(&answers, &answer) {
            cases.push(crashed_row(i, &format!("writing the answer: {failure}")));
            report(&answers_root, envelope_s, init_s, &cases);
            continue;
        }

        cases.push(ok_row(
            i, seconds, cts_in, cts_out, encoding_s, encrypt_s, decrypt_s, decoding_s,
        ));
        report(&answers_root, envelope_s, init_s, &cases);
    }

    solve::free(state);
    report(&answers_root, envelope_s, init_s, &cases);
    Ok(())
}

fn read_case(where_: &Path, p: &Point) -> Anyhow<Inputs> {
    let mut inp = Inputs::default();
    inp.xs = read_tensor::<f64>(where_, "xs", vec![p.N as i64])?;
    Ok(inp)
}

fn write_case(answers: &Path, answer: &Outputs) -> Anyhow<()> {
    fs::create_dir_all(answers)?;
    write_tensor(answers, "s", &answer.s)?;
    Ok(())
}
