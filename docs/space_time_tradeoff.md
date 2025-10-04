# Integrating Sublinear-Space Simulation Strategies into HVM2

This note investigates how Ryan Williams' $\sqrt{T \log T}$ time-space tradeoff for general computation might relate to the Higher-order Virtual Machine 2 (HVM2) architecture, and outlines concrete steps required before any implementation could be considered.

## Background

* **Williams' tradeoff** replaces stored intermediate state with controlled recomputation. The technique relies on recursively decomposing computations into tree-evaluation subproblems and carefully scheduling recomputation so that only $O(\sqrt{T \log T})$ space is ever needed.
* **HVM2** executes interaction nets using massively parallel reduction. Nodes are stored in 64-bit words with 3-bit tags, 29-bit ports, and a global redex bag orchestrating atomic rewrites.

Because HVM2 already emphasises locality and atomic interactions, it is a candidate platform for exploring recomputation-based space savings. However, applying Williams' method is far from straightforward.

## Gaps Between Williams' Simulation and HVM2

1. **Different computational models**: Williams' result is framed for multitape Turing machines (and, by reduction, bounded fan-in circuits). HVM2 works on interaction nets with different invariants and synchronization rules.
2. **Scheduler expectations**: Williams' technique assumes the ability to recompute arbitrary subproblems on demand. HVM2's runtime currently expects a monotonic reduction of nets, not repeated regeneration of subnets.
3. **Memory hierarchy usage**: GPU-oriented components rely on shared-memory caches and a leak mechanism. Repeated recomputation would increase contention and may negate existing throughput optimizations.

## Prerequisites for Implementation

* **Formal mapping** from Williams' recursive tree evaluation to interaction nets, ensuring that recomputed subnets can be generated without violating linearity constraints.
* **Redex classification** to flag which active pairs may be safely discarded and later re-instantiated.
* **Scheduler extensions** capable of storing lightweight continuations or hashes of discarded subnets, enabling deterministic reconstruction without storing the entire net.
* **Cost model tooling** that measures the recomputation versus storage break-even point, especially on GPUs where bandwidth and synchronization dominate.

## Recommended Next Steps

1. **Prototype on a restricted fragment**: Start with a subset of nets (e.g., tree-shaped, read-only inputs) and extend the runtime with an experimental recompute flag controlling when subnets are dropped.
2. **Develop instrumentation**: Add metrics to track peak node usage, recomputation frequency, and redex contention. This data will validate whether theoretical gains manifest in practice.
3. **Explore hybrid strategies**: Combine the existing LEAK mechanism with lazy recomputation, storing compact summaries (such as hashes or evaluators) instead of full nodes.
4. **Document invariants**: Record how recomputation interacts with referential transparency and constant-space tail recursion to prevent regressions in correctness.

## Prototype progress

The `tree-pebble` command implements a user-space experiment for Ryan Williams' tree-evaluation recomputation. It interprets JSON
instances with a bounded cache that stores a square-root number of subtree results, eagerly evicting and recomputing as required.
While it does not yet mutate interaction nets directly, the evaluator exposes cache-size knobs and records peak cache and stack
usage—instrumentation that will inform future scheduler integrations.

## Challenges

* **Correctness guarantees**: Any recomputation policy must preserve the confluence and determinism of interaction nets, which may require new proofs.
* **Parallel interference**: Massive parallelism increases the probability that multiple workers attempt to recompute the same subnet, leading to redundant work unless deduplicated.
* **Engineering cost**: Implementing the scheduling, instrumentation, and verification layers is substantial and should precede attempts to implement the full Williams tradeoff.

## Conclusion

In its current form, HVM2 does not implement state-of-the-art asymptotic time-space tradeoffs such as Williams' $\sqrt{T \log T}$ simulation. Realizing such improvements requires foundational research to reconcile the recomputation-heavy strategy with interaction net semantics and HVM2's parallel runtime. This document captures the research roadmap so that future contributors can evaluate feasibility before attempting a production implementation.
