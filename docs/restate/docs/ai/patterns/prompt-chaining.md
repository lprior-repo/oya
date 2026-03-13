# Prompt Chaining

Source: https://docs.restate.dev/ai/patterns/prompt-chaining

Build fault-tolerant processing pipelines with automatic retries and recovery.

Build fault-tolerant processing pipelines where each step transforms the previous step's output.
If any step fails, Restate automatically resumes from that exact point.

For example:

```
+----------+    +---------------+    +-------------+    +--------------+
|  Input   | => | Extract       | => | Sort        | => | Format as    |
| Message  |    | Metrics       |    | Results     |    | Table        |
+----------+    +---------------+    +-------------+    +--------------+
```

## How does Restate help?

The benefits of using Restate here are:

* **Automatic retries** of failed tasks: LLM API down, timeouts, infrastructure failures, etc.
* **Recovery of previous progress**: After a failure, Restate recovers the progress the execution did before the crash.
* Works with **any LLM SDK** (Vercel AI, LangChain, LiteLLM, etc.) and **any programming language** supported by Restate (TypeScript, Python, Go, etc.).

## Example

Wrap each step in the chain with `ctx.run()` to ensure fault tolerance and automatic recovery. Restate uses durable execution to persist the result of each step as it completes, so if any step fails, Restate will retry from that exact point without losing previous progress or re-executing completed steps.

<CodeGroup>
  ```ts TypeScript {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/typescript-patterns/src/chaining.ts#here"}  theme={null}
  async function process(ctx: Context, report: { message: string }) {
    // Step 1: Extract metrics
    const extract = await ctx.run(
      "Extract metrics",
      // Use your preferred LLM SDK here
      async () =>
        llmCall(`Extract numerical values and their metrics from the text. 
              Format as 'Metric Name: Value' per line. Input: ${report.message}`),
      { maxRetryAttempts: 3 },
    );

    // Step 2: Process the result from Step 1
    const sortedMetrics = await ctx.run(
      "Sort metrics",
      async () =>
        llmCall(`Sort lines in descending order by value: ${extract.text}`),
      { maxRetryAttempts: 3 },
    );

    // Step 3: Format as table
    const table = await ctx.run(
      "Format as table",
      async () =>
        llmCall(`Format the data as a markdown table: ${sortedMetrics.text}`),
      { maxRetryAttempts: 3 },
    );

    return table.text;
  }
  ```

  ```python Python {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/python-patterns/app/chaining.py?collapse_prequel"}  theme={null}
  call_chaining_svc = restate.Service("CallChainingService")


  @call_chaining_svc.handler()
  async def process(ctx: restate.Context, report: Report) -> str | None:
      """Sequentially chains multiple LLM calls, each transforming the prior output."""

      # Step 1: Extract metrics
      extract = await ctx.run_typed(
          "Extract metrics",
          llm_call,  # Use your preferred LLM SDK here
          RunOptions(max_attempts=3),  # Avoid infinite retries
          messages=f"""Extract numerical values and their metrics from the text. 
          Format as 'Metric Name: Value' per line. Input: {report.message}""",
      )

      # Step 2: Sort by value
      sorted_metrics = await ctx.run_typed(
          "Sort metrics",
          llm_call,
          RunOptions(max_attempts=3),
          messages=f"Sort lines in descending order by value: {extract}",
      )

      # Step 3: Format as table
      table = await ctx.run_typed(
          "Format as table",
          llm_call,
          RunOptions(max_attempts=3),
          messages=f"Format the data as a markdown table:{sorted_metrics}",
      )

      return table.content
  ```
</CodeGroup>

View on GitHub: [TS](https://github.com/restatedev/ai-examples/blob/typescript_patterns/typescript-patterns/src/chaining.ts) /
[Python](https://github.com/restatedev/ai-examples/blob/main/python-patterns/app/chaining.py)

The Restate UI shows how each step in the chain is executed and persisted:

<img alt="Chaining LLM calls - UI" />

<Tip>
  This pattern is implementable with any of our SDKs and any AI SDK.
  If you need help with a specific SDK, please reach out to us via [Discord](https://discord.restate.dev) or [Slack](https://slack.restate.dev).
</Tip>

<Accordion title="Run the example">
  <Steps>
    <Step title="Requirements">
      * AI SDK of your choice (e.g., OpenAI, LangChain, Pydantic AI, LiteLLM, etc.) to make LLM calls.
      * API key for your model provider.
    </Step>

    <Step title="Download the example">
      <CodeGroup>
        ```shell TypeScript theme={null}
        git clone https://github.com/restatedev/ai-examples.git &&
        cd typescript-patterns &&
        npm install
        ```

        ```shell Python theme={null}
        git clone https://github.com/restatedev/ai-examples.git &&
        cd python-patterns
        ```
      </CodeGroup>
    </Step>

    <Step title="Start the Restate Server">
      ```shell theme={null}
      restate-server
      ```
    </Step>

    <Step title="Start the Service">
      Export the API key of your model provider as an environment variable and then start the agent. For example, for OpenAI:

      <CodeGroup>
        ```shell TypeScript theme={null}
        export OPENAI_API_KEY=your_openai_api_key
        npm run dev
        ```

        ```shell Python theme={null}
        export OPENAI_API_KEY=your_openai_api_key
        uv run .
        ```
      </CodeGroup>
    </Step>

    <Step title="Register the services">
      <Tabs>
        <Tab title="UI">
          <img alt="Service Registration" />
        </Tab>

        <Tab title="CLI">
          ```shell theme={null}
          restate deployments register localhost:9080
          ```
        </Tab>
      </Tabs>
    </Step>

    <Step title="Send a request">
      In the UI (`http://localhost:9070`), click on the `process` handler of the `CallChainingService` to open the playground and send a default request:

      <img alt="Chaining LLM calls - UI" />
    </Step>

    <Step title="Check the Restate UI">
      You see in the Invocations Tab of the UI how the LLM is called multiple times, and how each result is persisted in Restate:

      <img alt="Chaining LLM calls - UI" />
    </Step>
  </Steps>
</Accordion>