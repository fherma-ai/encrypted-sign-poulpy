# The challenge polynomial, on Poulpy

The same answer as `solution/`, with one thing changed: the polynomial.

`solution/` is Poulpy's own reading of the task. The specification promises
every examined value stays 0.02 away from zero, so it interpolates a ramp that
saturates inside that gap and follows it with a Chebyshev series of degree 511
— half the degree, and the right answer everywhere a verdict is read.

This directory evaluates instead the series the OpenFHE, FIDESlib and DESILO
answers evaluate: the challenge-winning approximation of sign itself, degree
1023, coefficients unchanged. It is the slower of the two by construction, and
it is here so that one panel of the board is the same mathematics as the
others — what separates its number from theirs is the library alone.

Both are honest answers to the specification. The first is what somebody
writing for this specification would do; the second is what a comparison needs.
