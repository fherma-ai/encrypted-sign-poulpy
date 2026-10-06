# The same answer at a secret the standard's table is about

`solution/` uses a sparse ternary secret: Hamming weight 192 out of a ring of
2^14. The envelope then checks the modulus against the HE standard's table for
128-bit classic, which allows 438 bits at that ring — and that table is
computed for a *uniform* ternary secret, where about two thirds of the
coefficients are non-zero. Applying it to a secret with 192 of 16384 non-zero
is applying the wrong row.

This directory is the same answer with the secret at uniform ternary density,
so the table the envelope checks is the table the parameters are about. It is
here to say what the sparse secret was worth: if the two measure the same, the
sparseness bought nothing and `solution/` can adopt this; if they differ, the
difference is the part of Poulpy's number that rests on a weaker assumption
than OpenFHE's, which uses a uniform ternary secret by default.

Neither is a claim that the sparse parameters are insecure. Saying that needs
the lattice estimator run for h = 192, which is work for somebody who can read
the result — not for a benchmark.
