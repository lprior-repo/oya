//! Regression test for node drag disappearing bug
//!
//! This test verifies that nodes don't disappear when dragged.
//! Bug: When clicking and holding on a node, it would disappear.
//!
//! Run with `cargo test --test node_drag_regression -- --ignored` when the server is running.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]

use playwright::Playwright;
use std::path::Path;

const DEFAULT_APP_URL: &str = "http://localhost:8081";

fn app_url() -> Option<String> {
    if std::env::var("E2E").ok().as_deref() != Some("1") {
        return None;
    }
    Some(std::env::var("E2E_BASE_URL").map_or(DEFAULT_APP_URL.to_string(), |value| value))
}

/// Test: Node should remain visible when starting a drag operation
#[tokio::test]
#[ignore = "Requires running server at localhost:8081"]
async fn regression_node_should_not_disappear_when_drag_starts(
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(app_url) = app_url() else {
        eprintln!("Skipping E2E test; set E2E=1 to enable.");
        return Ok(());
    };
    let playwright = Playwright::initialize().await?;
    let browser = playwright.chromium().launcher().launch().await?;
    let context = browser.context_builder().build().await?;
    let page = context.new_page().await?;

    // Load app
    page.goto_builder(&app_url).goto().await?;

    // Take diagnostic screenshot after initial load
    page.wait_for_timeout(10000.0).await;
    page.screenshot_builder()
        .path(Path::new("regression_1_initial.png").to_path_buf())
        .screenshot()
        .await?;

    // Get page content for debugging
    let content = page.content().await?;
    println!("Page content after 10s: {} bytes", content.len());
    println!("Contains 'aside': {}", content.contains("aside"));
    println!("Contains 'main': {}", content.contains("main"));
    println!("Contains 'wasm': {}", content.contains("wasm"));
    println!("Contains 'data-node-id': {}", content.contains("data-node-id"));

    // Check for JavaScript errors
    let errors: serde_json::Value = page
        .evaluate(
            r#"
        () => {
            const errors = window.__dxErrors || [];
            return {
                errorCount: errors.length,
                errors: errors.slice(0, 10)
            };
        }
    "#,
            None::<()>,
        )
        .await?;
    println!("JavaScript errors: {:?}", errors);

    // Check if WASM loaded
    let wasm_status: serde_json::Value = page
        .evaluate(
            r#"
        () => {
            return {
                wasmModule: typeof WebAssembly !== 'undefined',
                mainDiv: !!document.querySelector('main'),
                mainChildren: document.querySelector('main')?.children.length || 0,
                bodyChildren: document.body.children.length,
                bodyInnerHTML: document.body.innerHTML.substring(0, 500)
            };
        }
    "#,
            None::<()>,
        )
        .await?;
    println!("WASM/DOM status: {:?}", wasm_status);

    // Wait for app to fully load with longer timeout
    let wait_result =
        page.wait_for_selector_builder("aside").timeout(30000.0).wait_for_selector().await;

    if wait_result.is_err() {
        page.screenshot_builder()
            .path(Path::new("regression_2_timeout.png").to_path_buf())
            .screenshot()
            .await?;
        println!("Timeout waiting for aside selector - returning early for diagnostics");
        // Return error but with diagnostics captured
        return Err("Timeout waiting for aside - check screenshots for diagnostics".into());
    }

    page.wait_for_selector_builder("input[placeholder='Search nodes...']")
        .timeout(10000.0)
        .wait_for_selector()
        .await?;

    // Add a node
    page.click_builder("text=HTTP Trigger").click().await?;
    let node = page
        .wait_for_selector_builder("div[data-node-id]")
        .timeout(10000.0)
        .wait_for_selector()
        .await?;

    // Get initial position - handle Option properly
    let node_ref = match node.as_ref() {
        Some(elem) => elem,
        None => {
            page.screenshot_builder()
                .path(Path::new("regression_no_element.png").to_path_buf())
                .screenshot()
                .await?;
            return Err("Node element not found".into());
        }
    };
    let initial_box = node_ref.bounding_box().await?;
    println!("Initial node position: {:?}", initial_box);

    let box_rect = match initial_box {
        Some(rect) => rect,
        None => {
            page.screenshot_builder()
                .path(Path::new("regression_no_bounding_box.png").to_path_buf())
                .screenshot()
                .await?;
            return Err("Node should have a bounding box".into());
        }
    };
    let _center_x = box_rect.x + box_rect.width / 2.0;
    let _center_y = box_rect.y + box_rect.height / 2.0;

    // Simulate mouse down (start drag) using JavaScript
    let result: serde_json::Value = page
        .evaluate(
            r#"
        () => {
            const canvas = document.querySelector('main');
            if (!canvas) return { error: 'canvas not found' };

            // Create and dispatch mousedown event on the node-card
            const nodeCard = document.querySelector('div[data-node-id]');
            if (!nodeCard) return { error: 'node not found' };

            const rect = nodeCard.getBoundingClientRect();
            const centerX = rect.left + rect.width / 2;
            const centerY = rect.top + rect.height / 2;

            const mousedownEvent = new MouseEvent('mousedown', {
                bubbles: true,
                cancelable: true,
                clientX: centerX,
                clientY: centerY,
                button: 0
            });

            nodeCard.dispatchEvent(mousedownEvent);

            // Check if node still exists
            return {
                success: true,
                nodeExists: !!document.querySelector('div[data-node-id]'),
                nodeVisible: document.querySelector('div[data-node-id]')?.offsetParent !== null
            };
        }
    "#,
            None::<()>,
        )
        .await?;

    println!("After mousedown via JS: {:?}", result);

    // Give time for any reactivity to process
    page.wait_for_timeout(500.0).await;

    // Check if node still exists
    let node_after = page.query_selector("div[data-node-id]").await?;
    println!("Node after mousedown: {:?}", node_after.is_some());

    if node_after.is_none() {
        page.screenshot_builder()
            .path(Path::new("regression_node_disappeared.png").to_path_buf())
            .screenshot()
            .await?;

        let content = page.content().await?;
        println!("Page content length: {}", content.len());
    }

    assert!(node_after.is_some(), "REGRESSION: Node disappeared after mousedown! This is the bug.");

    if let Some(node_element) = node_after {
        let box_after = node_element.bounding_box().await?;
        println!("Node bounding box after mousedown: {:?}", box_after);
        assert!(box_after.is_some(), "Node should still have a bounding box");
    }

    browser.close().await?;
    Ok(())
}

/// Test: Node should move when dragged, not disappear
#[tokio::test]
#[ignore = "Requires running server at localhost:8081"]
async fn regression_node_should_move_when_dragged_not_disappear(
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(app_url) = app_url() else {
        eprintln!("Skipping E2E test; set E2E=1 to enable.");
        return Ok(());
    };
    let playwright = Playwright::initialize().await?;
    let browser = playwright.chromium().launcher().launch().await?;
    let context = browser.context_builder().build().await?;
    let page = context.new_page().await?;

    page.goto_builder(&app_url).goto().await?;

    page.wait_for_selector_builder("aside").timeout(60000.0).wait_for_selector().await?;

    page.click_builder("text=HTTP Trigger").click().await?;
    page.wait_for_selector_builder("div[data-node-id]")
        .timeout(10000.0)
        .wait_for_selector()
        .await?;

    let result: serde_json::Value = page
        .evaluate(
            r#"
        async () => {
            const nodeCard = document.querySelector('div[data-node-id]');
            if (!nodeCard) return { error: 'node not found' };

            const rect = nodeCard.getBoundingClientRect();
            const startX = rect.left + rect.width / 2;
            const startY = rect.top + rect.height / 2;
            const endX = startX + 100;
            const endY = startY + 50;

            const mousedownEvent = new MouseEvent('mousedown', {
                bubbles: true,
                cancelable: true,
                clientX: startX,
                clientY: startY,
                button: 0
            });
            nodeCard.dispatchEvent(mousedownEvent);

            await new Promise(r => setTimeout(r, 50));

            const mousemoveEvent = new MouseEvent('mousemove', {
                bubbles: true,
                cancelable: true,
                clientX: endX,
                clientY: endY
            });
            document.dispatchEvent(mousemoveEvent);

            await new Promise(r => setTimeout(r, 50));

            const mouseupEvent = new MouseEvent('mouseup', {
                bubbles: true,
                cancelable: true,
                clientX: endX,
                clientY: endY,
                button: 0
            });
            document.dispatchEvent(mouseupEvent);

            await new Promise(r => setTimeout(r, 100));

            const finalNode = document.querySelector('div[data-node-id]');
            return {
                nodeExists: !!finalNode,
                nodeVisible: finalNode?.offsetParent !== null,
                nodeRect: finalNode ? finalNode.getBoundingClientRect() : null
            };
        }
    "#,
            None::<()>,
        )
        .await?;

    println!("Drag simulation result: {:?}", result);

    let node_after = page.query_selector("div[data-node-id]").await?;

    if node_after.is_none() {
        page.screenshot_builder()
            .path(Path::new("regression_drag_disappeared.png").to_path_buf())
            .screenshot()
            .await?;
    }

    assert!(node_after.is_some(), "REGRESSION: Node disappeared during drag!");

    browser.close().await?;
    Ok(())
}

/// Test: Node visibility after click (before drag)
#[tokio::test]
#[ignore = "Requires running server at localhost:8081"]
async fn node_should_be_visible_after_click() -> Result<(), Box<dyn std::error::Error>> {
    let Some(app_url) = app_url() else {
        eprintln!("Skipping E2E test; set E2E=1 to enable.");
        return Ok(());
    };
    let playwright = Playwright::initialize().await?;
    let browser = playwright.chromium().launcher().launch().await?;
    let context = browser.context_builder().build().await?;
    let page = context.new_page().await?;

    page.goto_builder(&app_url).goto().await?;

    page.wait_for_selector_builder("aside").timeout(60000.0).wait_for_selector().await?;

    page.click_builder("text=HTTP Trigger").click().await?;
    page.wait_for_selector_builder("div[data-node-id]")
        .timeout(10000.0)
        .wait_for_selector()
        .await?;

    page.click_builder("div[data-node-id]").click().await?;

    page.wait_for_timeout(200.0).await;

    let node = page.query_selector("div[data-node-id]").await?;
    assert!(node.is_some(), "Node should exist after click");

    if let Some(n) = node {
        let box_rect = n.bounding_box().await?;
        assert!(box_rect.is_some(), "Node should be visible after click");
        println!("Node box after click: {:?}", box_rect);
    }

    browser.close().await?;
    Ok(())
}
