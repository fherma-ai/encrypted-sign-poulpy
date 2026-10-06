# The same circuit at OpenFHE's own parameters

Ring 2^15 and a uniform ternary secret — the ring the OpenFHE answer to this
specification uses, and the secret distribution it uses by default.

The other directories take the parameters the library can afford: a ring of
2^14, because the torus carries this circuit in 300 bits where a prime chain
needs 560, and originally a sparse secret of Hamming weight 192. Both are
legitimate and both are what somebody writing for this specification would
choose. Neither is comparable with OpenFHE on the question "how fast is the
library", because a smaller ring is less work and a sparser secret is less
noise — and the standard's table for 128-bit classic, which the envelope
checks against, is computed for a uniform ternary secret.

This directory removes both differences so that what is left is the library.
It is the slower of them by construction.
