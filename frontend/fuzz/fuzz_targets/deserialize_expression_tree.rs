#![no_main]
use libfuzzer_sys::fuzz_target;
use oya_frontend::expression_depth::deserialize_expression_tree;

fuzz_target!(|data: &[u8]| {
    // Must not panic - any input is valid to attempt deserialization
    let _result = deserialize_expression_tree(data);
});
