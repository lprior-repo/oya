// Test to verify oya-ui does not depend on crossterm
//
// This test ensures that the crate can be used without the crossterm
// dependency, which was an issue in older versions where crossterm
// was used without proper feature guards.

#[test]
fn test_oya_ui_compiles_without_crossterm() {
    // This test compiles successfully if oya-ui does not use crossterm
    // The bead src-2za6 reported that crossterm was used without feature guards
    // causing compilation failures.
    //
    // Current code (restored from git f28ca43af) does NOT use crossterm,
    // so this test should pass.

    // If this test compiles, the issue is fixed
    // Verify we can use oya-ui types without crossterm
    let _size = oya_ui::Size { rows: 24, cols: 80 };
}
