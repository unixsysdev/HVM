Higher-order Virtual Machine 2 (HVM2)
=====================================

**Higher-order Virtual Machine 2 (HVM2)** is a massively parallel [Interaction
Combinator](https://www.semanticscholar.org/paper/Interaction-Combinators-Lafont/6cfe09aa6e5da6ce98077b7a048cb1badd78cc76)
evaluator.

By compiling programs from high-level languages (such as Python and Haskell) to
HVM, one can run these languages directly on massively parallel hardware, like
GPUs, with near-ideal speedup.

HVM2 is the successor to [HVM1](https://github.com/HigherOrderCO/HVM1), a 2022
prototype of this concept. Compared to its predecessor, HVM2 is simpler, faster
and, most importantly, more correct. [HOC](https://HigherOrderCO.com/) provides
long-term support for all features listed on its [PAPER](./paper/HVM2.pdf).

Additional design notes collected during this exploration are available in the
repository under [docs/space_time_tradeoff.md](./docs/space_time_tradeoff.md).

This repository provides a low-level IR language for specifying the HVM2 nets
and a compiler from that language to C and CUDA. It is not meant for direct
human usage. If you're looking for a high-level language to interface with HVM2,
check [Bend](https://github.com/HigherOrderCO/Bend) instead.

Usage
-----

> DISCLAIMER: Windows is currently not supported, please use [WSL](https://learn.microsoft.com/en-us/windows/wsl/install) for now as a workaround.

First install the dependencies:
* If you want to use the C runtime, install a C-11 compatible compiler like GCC or Clang.
* If you want to use the CUDA runtime, install CUDA and nvcc (the CUDA compiler).
  - _HVM requires CUDA 12.x and currently only works on Nvidia GPUs._

Install HVM2:

```sh
cargo install hvm
```

There are multiple ways to run an HVM program:

```sh
hvm run    <file.hvm> # interpret via Rust
hvm run-c  <file.hvm> # interpret via C
hvm run-cu <file.hvm> # interpret via CUDA
hvm gen-c  <file.hvm> # compile to standalone C
hvm gen-cu <file.hvm> # compile to standalone CUDA
hvm tree-pebble <file.json> [--cache N]
```

All modes produce the same output. The compiled modes require you to compile the
generated file (with `gcc file.c -o file`, for example), but are faster to run.
The CUDA versions have much higher peak performance but are less stable. As a
rule of thumb, `gen-c` should be used in production.

### Experimental tree evaluation pebbling

The `tree-pebble` subcommand loads a tree-evaluation instance encoded as JSON
and simulates Ryan Williams' recomputation-based space savings using a bounded
cache. The evaluator keeps only a limited number of subtree values in memory and
recomputes evicted subtrees on demand, providing a practical hook for
space–time tradeoff experiments without touching the core interaction-net
runtime.

```sh
# Evaluate the example tree with the default square-root-sized cache
hvm tree-pebble examples/tree_eval.json

# Force a very small cache to observe additional recomputation
hvm tree-pebble examples/tree_eval.json --cache 2
```

The command reports the result alongside cache statistics so that you can gauge
how aggressively recomputation is being used. See
[docs/space_time_tradeoff.md](./docs/space_time_tradeoff.md) for research notes
on integrating the same idea with full interaction nets.

Language
--------

HVM is a low-level compile target for high-level languages. It provides a raw
syntax for wiring interaction nets. For example:

```javascript
@main = a
  & @sum ~ (28 (0 a))

@sum = (?(((a a) @sum__C0) b) b)

@sum__C0 = ({c a} ({$([*2] $([+1] d)) $([*2] $([+0] b))} f))
  &! @sum ~ (a (b $([+] $(e f))))
  &! @sum ~ (c (d e))
```

The file above implements a recursive sum. If that looks unreadable to you -
don't worry, it isn't meant to. [Bend](https://github.com/HigherOrderCO/Bend) is
the human-readable language and should be used both by end users and by languages
aiming to target the HVM. If you're looking to learn more about the core
syntax and tech, though, please check the [PAPER](./paper/HVM2.pdf).
