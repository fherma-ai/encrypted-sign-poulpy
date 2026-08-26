# Sign over Poulpy — one Chebyshev of the ramp

> **A FHERMA reference implementation on
> [Poulpy](https://github.com/poulpy-fhe/poulpy) (Apache-2.0)** — the first
> answer on the platform built on a second FHE library, measured against the
> OpenFHE answer to the same specification. Implements
> [`sign` / `f64@1.0.0`](https://fherma.io/kernels/sign) on the FHERMA
> kernel catalogue.

sign(x) for every element of an encrypted vector, x ∈ [−1, 1]. The
specification promises every examined value stays 0.02 away from zero, so
the circuit interpolates the ramp clamp(x / 0.02) — a degree-511 Chebyshev
polynomial evaluated in one Baby-Step Giant-Step pass, nine rescales of the
budget, no rotation keys. Torus width 330 bits at ring degree 16384, under
the toolchain's 128-bit floor.

```text
kernel sign<N: uint>(
    %xs: secret<tensor<N x f64>>,
) -> %s: secret<tensor<N x f64>>
```

## Layout

| | |
| --- | --- |
| [`solution/`](solution/) | the measured project: `src/solve.rs` (the answer), `config.jsonc` (the torus, in bits) |
| [`solution/README.md`](solution/README.md) | the contract: what is measured, what is reviewed, how to run |

## Running it

```sh
cd solution
docker run --rm -v "$PWD":/solution -w /solution fherma/poulpy:0.8.2 \
    sh -c "cargo build --release --offline \
           && install -m 0755 /opt/cargo-target/release/fherma-solution ./fherma-solution \
           && ./fherma-solution <point-dir>"
```

## License

Apache-2.0.
