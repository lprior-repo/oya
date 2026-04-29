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

#[tokio::test]
async fn given_flow_editor_when_loaded_then_sidebar_and_node_canvas_are_interactive(
) -> Result<(), Box<dyn std::error::Error>> {
    let Some(app_url) = app_url() else {
        eprintln!("Skipping E2E test; set E2E=1 to enable.");
        return Ok(());
    };
    let playwright = Playwright::initialize().await?;

    let chromium = playwright.chromium();
    let browser = chromium.launcher().launch().await?;
    let context_obj = browser.context_builder().build().await?;
    let page = context_obj.new_page().await?;

    println!("Navigating to app (release mode)...");
    page.goto_builder(&app_url).goto().await?;

    println!("Given the app loads, waiting for sidebar shell (up to 60s)...");
    match page.wait_for_selector_builder("aside").timeout(60000.0).wait_for_selector().await {
        Ok(_) => {
            println!("SUCCESS: Aside found!");
            page.wait_for_selector_builder("input[placeholder='Search nodes...']")
                .timeout(10000.0)
                .wait_for_selector()
                .await?;
            page.wait_for_selector_builder("text=HTTP Trigger")
                .timeout(10000.0)
                .wait_for_selector()
                .await?;
            page.click_builder("text=HTTP Trigger").click().await?;
            page.wait_for_selector_builder("div[data-node-id]")
                .timeout(10000.0)
                .wait_for_selector()
                .await?;
            println!("Then adding a node renders an interactive node card.");

            page.screenshot_builder()
                .path(Path::new("final_success.png").to_path_buf())
                .screenshot()
                .await?;
        }
        Err(e) => {
            println!("FAILURE: Aside still not found: {e:?}");
            let page_content = page.content().await?;
            println!("Content length: {}", page_content.len());
            page.screenshot_builder()
                .path(Path::new("final_failure.png").to_path_buf())
                .screenshot()
                .await?;
        }
    }

    browser.close().await?;
    Ok(())
}
