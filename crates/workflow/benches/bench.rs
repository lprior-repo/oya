// Main benchmark entry point
// Run individual benchmarks with:
//   cargo bench --bench loop_vs_iterator
//   cargo bench --bench clone_vs_arc
//   cargo bench --bench hashmap_vs_im
//   cargo bench --bench error_handling
//   cargo bench --bench vec_operations
//
// Run all benchmarks with:
//   cargo bench

mod clone_vs_arc;
mod error_handling;
mod hashmap_vs_im;
mod loop_vs_iterator;
mod vec_operations;
