import { defineConfig, devices } from "@playwright/test";

export default defineConfig({
  testDir: "./e2e",
  outputDir: ".playwright/test-results",
  timeout: 60_000,
  expect: {
    timeout: 15_000,
  },
  fullyParallel: false,
  retries: process.env.CI ? 1 : 0,
  workers: 1,
  reporter: [["list"], ["html", { open: "never", outputFolder: ".playwright/report" }]],
  use: {
    baseURL: process.env.E2E_BASE_URL ?? "http://127.0.0.1:8081",
    trace: "retain-on-failure",
    screenshot: "only-on-failure",
    video: "off",
    reducedMotion: "reduce",
  },
  webServer: {
    command: "node_modules/.bin/tailwindcss -i ./assets/tailwind.input.css -o ./assets/tailwind.css && env -u RUSTC_WRAPPER dx build --platform web --release && mkdir -p target/dx/oya-frontend/release/web/public/assets && cp assets/tailwind.css target/dx/oya-frontend/release/web/public/assets/tailwind.css && rm -rf dist && cp -r target/dx/oya-frontend/release/web/public dist && python3 -m http.server 8081 --directory dist",
    url: "http://127.0.0.1:8081",
    timeout: 240_000,
    reuseExistingServer: !process.env.CI,
    stdout: "pipe",
    stderr: "pipe",
  },
  projects: [
    {
      name: "chromium",
      use: { ...devices["Desktop Chrome"] },
    },
  ],
});
