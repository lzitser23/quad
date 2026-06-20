# Defer generating the TS IPC contract from Rust

The IPC DTOs (`ipc.rs`) and the action table (`actions.rs`) are mirrored by hand in TS
(`web/src/lib/types.ts`, `web/src/lib/actions.ts`). We considered making Rust the single source and
generating the TS with `specta`/`tauri-specta`, and decided **not to, for now**.

The contract is small and churns rarely; the TS side also layers UI-only metadata (category, blurb,
preview regions) the Rust side doesn't have. A codegen step adds build weight and a generated-file
workflow for little payoff at this size. Revisit if the DTOs or action set start changing often, or
if a drift bug actually bites — at which point generating the shared shape (Rust → TS base, UI
metadata layered on top) becomes worth it.

This ADR exists so an architecture review doesn't re-propose the codegen each pass: the duplication
is known and accepted.
