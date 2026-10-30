# GPU feasibility report

Date: 2026-10-30. Decision: **ship CPU as the only computation backend**.
`--device gpu` is accepted for forward compatibility and falls back to CPU
with a warning. This is the option the specification allows when GPU
acceleration is not viable.

## Hardware probe

`vulkaninfo --summary` on this machine reports:

| Device | Type | Driver |
|---|---|---|
| AMD Radeon RX 6800M (RADV NAVI22) | discrete | Mesa 26.2.4, Vulkan 1.4.354 |
| AMD Radeon 680M (RADV REMBRANDT) | integrated | Mesa 26.2.4, Vulkan 1.4.354 |
| llvmpipe (LLVM 22.1.8) | CPU | Mesa 26.2.4 |

Both AMD devices expose `shaderFloat64 = true`, so the hardware can do f64
arithmetic. A Vulkan adapter is therefore available for a `wgpu` backend.

## Why a `wgpu` backend was not shipped

1. **WGSL has no `f64` type.** `wgpu` shaders are written in WGSL, which only
   guarantees 32-bit floats. Matching the Python/Rust objective within `1e-9`
   would require manual double-double or fixed-point emulation in the shader,
   which is both slower and a large correctness risk. The tasklist requires
   100% decision parity, not approximate GPU scoring.
2. **The search is sequential, not batched.** Quench and ILS evaluate a
   candidate, accept or reject it, then build the next move from the updated
   state. The delta tables are indexed by the current permutation. A GPU can
   only help by scoring large independent batches, which would change the
   sequence of decisions unless the CPU still evaluates them one at a time.
3. **The CPU is already fast enough for the tuning budget.** A full
   200k-iteration, 28-restart, 8-ILS run finishes in roughly one to two
   minutes, and the Rust sweeps reach the same optimum at 25k iterations.
   Kernel launch, buffer transfer, and f64 emulation would dominate the work
   that actually remains.
4. **Parity is more valuable than a speculative speedup.** Shipping two
   numerics paths multiplies the test surface for a generator whose output is
   already at the observed plateau.

