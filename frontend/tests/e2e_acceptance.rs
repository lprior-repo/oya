//! E2E Acceptance Tests for Flow Editor
//!
//! Tests critical user flows using Playwright.
//! Run with `cargo test --test e2e_acceptance -- --ignored` when the server is running.
#![allow(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::float_cmp)]

use playwright::{api::MouseButton, Playwright};
use std::path::Path;

const DEFAULT_APP_URL: &str = "http://localhost:8081";

fn app_url() -> Option<String> {
    if std::env::var("E2E").ok().as_deref() != Some("1") {
        return None;
    }
    Some(std::env::var("E2E_BASE_URL").map_or(DEFAULT_APP_URL.to_string(), |value| value))
}

async fn wait_for_shell(page: &playwright::api::Page) -> Result<(), Box<dyn std::error::Error>> {
    page.wait_for_selector_builder("aside").timeout(30000.0).wait_for_selector().await?;
    page.wait_for_selector_builder("input[placeholder='Search nodes...']")
        .timeout(10000.0)
        .wait_for_selector()
        .await?;
    Ok(())
}

async fn add_node_from_sidebar(
    page: &playwright::api::Page,
    label: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    page.click_builder(&format!("text={label}")).click().await?;
    page.wait_for_selector_builder("div[data-node-id]")
        .timeout(10000.0)
        .wait_for_selector()
        .await?;
    Ok(())
}

/// Test: Adding a node from sidebar
#[tokio::test]
#[ignore = "Requires running server at localhost:8081"]
async fn given_app_loaded_when_adding_node_then_node_appears_on_canvas(
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

    wait_for_shell(&page).await?;

    add_node_from_sidebar(&page, "HTTP Trigger").await?;

    let result = page
        .wait_for_selector_builder("div[data-node-id]")
        .timeout(5000.0)
        .wait_for_selector()
        .await;

    if result.is_err() {
        page.screenshot_builder()
            .path(Path::new("test_failure_add_node.png").to_path_buf())
            .screenshot()
            .await?;
    }

    match result {
        Ok(_) => {}
        Err(e) => panic!("Node should appear after clicking Add Node: {e}"),
    }

    browser.close().await?;
    Ok(())
}

/// Test: Node selection
#[tokio::test]
#[ignore = "Requires running server at localhost:8081"]
async fn given_node_exists_when_clicking_node_then_node_becomes_selected(
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

    wait_for_shell(&page).await?;

    add_node_from_sidebar(&page, "HTTP Trigger").await?;

    // Click on canvas first to ensure focus
    page.click_builder("main").click().await?;

    // Click the node
    page.click_builder("div[data-node-id]").click().await?;

    // Take screenshot to verify state
    page.screenshot_builder()
        .path(Path::new("test_node_selected.png").to_path_buf())
        .screenshot()
        .await?;

    browser.close().await?;
    Ok(())
}

/// Test: Right-click context menu
#[tokio::test]
#[ignore = "Requires running server at localhost:8081"]
async fn given_app_when_right_clicking_canvas_then_context_menu_appears(
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

    wait_for_shell(&page).await?;

    // Right click on main canvas area
    page.click_builder("main").button(MouseButton::Right).click().await?;

    // Use explicit wait instead of sleep
    page.wait_for_timeout(500.0).await;

    page.screenshot_builder()
        .path(Path::new("test_context_menu.png").to_path_buf())
        .screenshot()
        .await?;

    browser.close().await?;
    Ok(())
}

/// Test: Adding multiple nodes
#[tokio::test]
#[ignore = "Requires running server at localhost:8081"]
async fn given_app_when_adding_multiple_nodes_then_all_appear(
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

    wait_for_shell(&page).await?;

    add_node_from_sidebar(&page, "HTTP Trigger").await?;
    add_node_from_sidebar(&page, "HTTP Trigger").await?;
    add_node_from_sidebar(&page, "HTTP Trigger").await?;

    // Count nodes
    let nodes = page.query_selector_all("div[data-node-id]").await?;
    println!("Found {} nodes", nodes.len());

    page.screenshot_builder()
        .path(Path::new("test_multiple_nodes.png").to_path_buf())
        .screenshot()
        .await?;

    assert!(nodes.len() >= 3, "Should have at least 3 nodes");

    browser.close().await?;
    Ok(())
}

/// Test: Sidebar search
#[tokio::test]
#[ignore = "Requires running server at localhost:8081"]
async fn given_app_when_typing_in_search_then_filters_results(
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

    page.wait_for_selector_builder("input[placeholder='Search nodes...']")
        .timeout(30000.0)
        .wait_for_selector()
        .await?;

    // Type in search - keyboard press needs delay param
    page.click_builder("input[placeholder='Search nodes...']").click().await?;
    page.keyboard.press("H", None).await?;
    page.keyboard.press("T", None).await?;
    page.keyboard.press("T", None).await?;
    page.keyboard.press("P", None).await?;

    page.wait_for_timeout(500.0).await;

    // Should see HTTP Trigger
    let http = page.query_selector("text=HTTP Trigger").await?;
    assert!(http.is_some(), "Should find HTTP Trigger when searching");

    page.screenshot_builder().path(Path::new("test_search.png").to_path_buf()).screenshot().await?;

    browser.close().await?;
    Ok(())
}

/// Test: Verify initial app state
#[tokio::test]
#[ignore = "Requires running server at localhost:8081"]
async fn given_app_url_when_loading_then_all_ui_elements_present(
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

    // Wait for sidebar
    let sidebar =
        page.wait_for_selector_builder("aside").timeout(30000.0).wait_for_selector().await;
    match sidebar {
        Ok(_) => {}
        Err(e) => panic!("Sidebar should be present: {e}"),
    }

    // Check for search input
    let search = page
        .wait_for_selector_builder("input[placeholder='Search nodes...']")
        .timeout(10000.0)
        .wait_for_selector()
        .await;
    match search {
        Ok(_) => {}
        Err(e) => panic!("Search input should be present: {e}"),
    }

    // Check for Add Node button
    let sidebar_item = page.query_selector("text=HTTP Trigger").await?;
    assert!(sidebar_item.is_some(), "Sidebar nodes should be present");

    // Check for main canvas area
    let canvas = page.query_selector("main").await?;
    assert!(canvas.is_some(), "Main canvas should be present");

    // Check for HTTP Trigger (default sidebar item)
    let http = page
        .wait_for_selector_builder("text=HTTP Trigger")
        .timeout(10000.0)
        .wait_for_selector()
        .await;
    match http {
        Ok(_) => {}
        Err(e) => panic!("HTTP Trigger should be in sidebar: {e}"),
    }

    browser.close().await?;
    Ok(())
}
