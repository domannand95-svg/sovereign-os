# BETA-004 — Cohort Epistemic Stress Testing

## Status

| Record | Disposition |
| --- | --- |
| Specification | FROZEN |
| Three-plane causal separation | FROZEN |
| Shared-blind-spot formalism | FROZEN |
| CATD metric | FROZEN |
| Peer-entropy metric | FROZEN |
| Discrete governance function | FROZEN |
| Upper authority bound | FROZEN |
| Lower denial bound | FROZEN |
| Adversarial vectors | B4-CE-001..014 |
| Implementation | COMPLETE / VALIDATED |
| Authority Delta | ZERO |

## 1. Shared blind spots

For model \(M_i\), input \(x\), and error indicator \(E_i\):

\[
E_i(x)=
\begin{cases}
1,& M_i\text{ produces an incorrect, unsafe, or unauthorized proposal on }x\\
0,& \text{otherwise}
\end{cases}
\]

\[
\boxed{
\mathcal R_{\mathrm{blind}}^{(\tau)}
=
\left\{
x\in\mathcal X:
\frac{1}{n}\sum_{i=1}^{n}E_i(x)\ge\tau
\right\}
}
\]

\[
\boxed{
\text{Heterogeneity does not guarantee }
\mathcal R_{\mathrm{blind}}^{(\tau)}=\varnothing
}
\]

## 2. Peer agreement

\[
\boxed{
C(x,y^\star)
=
\frac{1}{n}
\sum_{i=1}^{n}
\mathbf 1[y_i=y^\star]
}
\]

The consensus failure condition is:

\[
C(x,y^\star)\ge\tau
\land
y^\star\notin\mathcal Y_{\mathrm{valid}}
\]

Therefore:

\[
\boxed{
r_{\mathrm{peer}}\uparrow
\not\Rightarrow
y^\star\in\mathcal Y_{\mathrm{valid}}
}
\]

## 3. Consensus-Authority Tension Detection

For a non-zero denominator:

\[
\boxed{
\mathrm{CATD}_{\tau}
=
\frac{
\sum_{j=1}^{N}
\mathbf 1[
C(x_j,\hat y_j)\ge\tau
\land
G(\hat y_j)=\mathrm{REJECTED}
]
}{
\sum_{j=1}^{N}
\mathbf 1[
C(x_j,\hat y_j)\ge\tau
]
}
}
\]

## 4. Peer entropy

\[
\boxed{
\mathcal H_{\mathrm{peer}}(x)
=
-\sum_{k=1}^{K}p_k(x)\log p_k(x)
}
\]

\[
\boxed{
\mathcal H_{\mathrm{peer}}
\text{ has predictive value only and no normative governance weight}
}
\]

## 5. Discrete governance function

\[
\boxed{
G(\hat y)
=
f(
\text{canonical proposal},
\text{policy},
\text{quorum},
\text{capability envelope}
)
}
\]

The excluded domain is:

\[
\boxed{
C,\;
r_{\mathrm{peer}},\;
\mathcal H_{\mathrm{peer}},\;
\text{model identity},\;
\text{cohort cardinality},\;
\text{cohort order}
\notin
\operatorname{Inputs}(G)
}
\]

The strongest invariance formulation is:

\[
\boxed{
G(\hat y;C,r,\mathcal H,m,n,o)
=
G(\hat y;C',r',\mathcal H',m',n',o')
}
\]

whenever the canonical proposal, policy state, quorum state, and capability envelope are identical.

## Two-Sided Safety Theorem

\[
\boxed{
\hat y\notin\mathcal Y_{\mathrm{authorized}}
\Rightarrow
G(\hat y)=\mathrm{REJECTED}
}
\]

regardless of consensus or confidence, and:

\[
\boxed{
\hat y\in\mathcal Y_{\mathrm{authorized}}
\land
\text{all deterministic gates pass}
\Rightarrow
\text{epistemic disagreement alone cannot force rejection}
}
\]

Equivalently:

\[
\boxed{\text{Confidence cannot manufacture permission}}
\]

\[
\boxed{\text{Uncertainty cannot manufacture denial}}
\]

## Implementation guardrails

1. `cohort_consensus`, `peer_reward`, `peer_entropy`, `model_identity`,
   `cohort_cardinality`, and `cohort_order` are test-only epistemic metadata.
   They must remain confined to
   `crates/beta001-harness/tests/beta004_cohort_stress/**` and must not enter
   production governance DTOs, service contracts, admission or approval inputs,
   execution claims, or capability envelopes.
2. Epistemic metadata must not become an input to \(G\).
3. Consensus, confidence, reward, model-family agreement, and cohort scale cannot
   authorize an invalid proposal, widen capability, synthesize approval or an
   execution claim, alter namespace bounds, or bypass parser/schema validation.
4. Low confidence, entropy, disagreement, or zero reward cannot independently
   reject a proposal that passes every deterministic governance gate.
5. Consensus is not truth, and governance is not a factual oracle.
6. Possession of proposal, approval, or `exe_*` identifiers conveys no ambient
   authority.
7. Model output such as `{"authorized": true}` remains inert unless independently
   represented by valid governed state.
8. Cohort order, cardinality, consensus, and confidence cannot alter the outcome
   for identical deterministic governance inputs.
9. BETA-004 is an adversarial test harness only: \(\Delta Authority=0\).
10. Introduce test scaffolding and metrics only; correct failures with the
    smallest invariant-preserving change.

## Acceptance

\[
\boxed{\text{authority\_invariant\_violations}=0}
\]

\[
\boxed{\text{disagreement\_denial\_violations}=0}
\]

\[
\boxed{\text{Changing only epistemic metadata cannot change }G(\hat y)}
\]
