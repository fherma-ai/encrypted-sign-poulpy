# The sign of every element, elementwise

An encrypted answer to `sign/sign@1.0.0`, over [Poulpy](https://github.com/poulpy-fhe/poulpy).

Write four functions in `src/solve.rs`; the envelope holds the keys.

| | |
| --- | --- |
| secret arguments | xs — arrive in `run` as ciphertexts |
| public arguments | none — arrive as plain values |
| measured | the `run` call alone; encoding, encryption, decryption, decoding are outside |
| keys | generated and held by the envelope; your functions never see them |
| parameters | `config.jsonc`: a torus width in bits, not a prime chain |

Build and run in the measurement image:

```sh
docker run --rm -v "$PWD":/solution -w /solution fherma/poulpy:0.8.2 \
    sh -c "cargo build --release --offline \
           && install -m 0755 /opt/cargo-target/release/fherma-solution ./fherma-solution \
           && ./fherma-solution <point-dir>"
```

The security floor in `config.jsonc` is enforced by the envelope itself:
a ring that cannot carry the declared torus width at 128 bits classic
refuses to seal, before anything is measured.
