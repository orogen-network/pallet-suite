//! Builds the runtime WASM blob via `substrate-wasm-builder`.
//!
//! Only runs when the `std` feature is enabled (i.e. the normal host build
//! of the runtime crate). When the runtime is itself being compiled to
//! wasm32-unknown-unknown, `substrate-wasm-builder` is omitted from the
//! dependency graph and this script is a no-op.

#[cfg(feature = "std")]
fn main() {
    substrate_wasm_builder::WasmBuilder::new()
        .with_current_project()
        .export_heap_base()
        .import_memory()
        .build();
}

#[cfg(not(feature = "std"))]
fn main() {}
