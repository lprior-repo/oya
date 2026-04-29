# OYA Frontend

A client-side, zero-latency workflow engine that runs entirely in the browser via WebAssembly (WASM). This application allows users to visually design, manage, and execute automation workflows without needing a backend server.

## Overview

OYA Frontend brings the power of workflow automation (similar to n8n or Zapier) directly to the client. By leveraging Rust and Dioxus compiled to WASM, it offers high performance, privacy by default, and offline-capable execution.

## Features

*   **Visual Workflow Editor:** Drag-and-drop interface for creating and connecting nodes.
*   **Client-Side Execution:** The execution engine runs entirely in your browser using WASM. No data leaves your machine unless you configure an external request.
*   **Persistence:** Workflows are automatically saved to your browser's `localStorage`.
*   **Node Library:** A comprehensive set of nodes for various tasks:
    *   **Triggers:** Webhook, Schedule, Email.
    *   **Actions:** HTTP Request, Transform (JSON manipulation), Custom Code (JS), AI Model integration.
    *   **Logic:** Condition (If/Else), Switch, Loop, Merge.
    *   **Outputs:** Slack, Email, File Write.
*   **Zero Latency:** Instant execution feedback without server round-trips.
*   **Privacy Focused:** Your logic and credentials stay local.

## Architecture

The project is built using the **Autonomous Development Triangle** methodology, ensuring high quality through rigorous specifications and behavioral testing.

*   **Frontend Framework:** [Dioxus](https://dioxuslabs.com/) (Rust)
*   **Styling:** Tailwind CSS
*   **Runtime:** WebAssembly (wasm32-unknown-unknown)
*   **State Management:** Dioxus Signals & LocalStorage
*   **Execution Engine:** Custom graph traversal and execution logic in Rust.

## Getting Started

### Prerequisites

*   **Rust:** [Install Rust](https://www.rust-lang.org/tools/install)
*   **Dioxus CLI:** `cargo install dioxus-cli`
*   **Moon:** [Install MoonRepo](https://moonrepo.dev/) (Optional, but recommended for tasks)

### Running the Application

1.  **Clone the repository:**
    ```bash
    git clone https://github.com/yourusername/new-app.git
    cd new-app
    ```

2.  **Run the development server:**
    ```bash
    # Using Moon (Recommended)
    moon run :serve

    # Or directly with Dioxus CLI
    dx serve --platform web --port 8081
    ```

3.  **Open in Browser:**
    Navigate to `http://localhost:8081`

## Project Structure

*   `src/main.rs`: Application entry point and main UI layout.
*   `src/ui/`: UI components (Toolbar, Sidebar, Node components, Minimap).
*   `src/graph/`: Core graph logic, node definitions, and execution engine.
*   `specs/`: Detailed behavioral specifications (`flow-wasm-v1.yaml`).

## Development

This project follows a strict quality gate process.
*   **Specs:** `specs/flow-wasm-v1.yaml` defines the expected behavior.
*   **Linting:** `moon run :clippy`
*   **Formatting:** `moon run :fmt`
*   **Tests:** `moon run :test`

## Quality Gate & Tooling

The repository includes a suite of tools to enforce quality standards and validate the implementation against specifications. These tools are located in `src/bin/`.

*   **Spec Linter:** Validates specification files against quality rules.
    ```bash
    cargo run --bin spec-linter -- specs/flow-wasm-v1.yaml
    ```
*   **Scenario Runner:** Executes behavioral scenarios against the running application.
    ```bash
    cargo run --bin scenario-runner --scenarios-path ../scenarios-vault/flow-wasm
    ```
*   **Quality Gate:** Orchestrates the full validation pipeline.
    ```bash
    cargo run --bin quality-gate full
    ```
*   **Dashboard:** View quality metrics and session history.
    ```bash
    cargo run --bin quality-dashboard summary
    ```

## License

[MIT License](LICENSE)
