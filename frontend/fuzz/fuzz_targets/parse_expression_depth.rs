#![no_main]
use libfuzzer_sys::fuzz_target;
use oya_frontend::expression_depth::parse_expression_depth;

fuzz_target!(|data: &[u8]| {
    // Must not panic - any input is valid to attempt parsing
    let _result = parse_expression_depth(data);
});
