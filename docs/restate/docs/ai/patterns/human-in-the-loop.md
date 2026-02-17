# Human-in-the-loop steps

Source: https://docs.restate.dev/ai/patterns/human-in-the-loop

Build resilient human approval steps in your agent workflows.

Build resilient workflows that include human approval steps or external signals without worrying about failures or interruptions.

Sometimes you need to include a human evaluator, approval step, or another external signal in an agentic workflow.
With Restate, you can model this by creating a Durable Promise and awaiting its completion (via a callback). Without worrying about failures or interruptions.

## How does Restate help?

The benefits of using Restate here are:

* **Durable promises**: Create promises that survive process restarts and failures
* **Automatic recovery**: Resume exactly where you left off after any interruption
* Works with **any LLM SDK** (Vercel AI, LangChain, LiteLLM, etc.) and **any programming language** supported by Restate (TypeScript, Python, Go, etc.).
* **Serverless-friendly waiting**: Suspend execution during approval wait times - pay for active work, not idle time:

<img />

## Example

Use `ctx.awakeable()` to create durable promises that can be resolved externally. The workflow suspends during the wait and resumes automatically when the promise is resolved.

<CodeGroup>
  ```ts TypeScript {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/typescript-patterns/src/human-in-the-loop.ts#here"}  theme={null}
  const tools = {
    getHumanReview: tool({
      description: "Request human review if policy violation is uncertain.",
      inputSchema: z.object({}),
    }),
  };

  async function moderate(ctx: Context, { message }: { message: string }) {
    const prompt = `You are a content moderation agent. Decide if the content violates policy: ${message}`;
    const { text, toolCalls } = await ctx.run(
      "LLM call",
      // Use your preferred LLM SDK here
      async () => llmCall(prompt, tools),
      { maxRetryAttempts: 3 },
    );

    if (toolCalls?.[0]?.toolName === "getHumanReview") {
      // Create a recoverable approval promise
      const approval = ctx.awakeable<string>();
      await ctx.run("Ask review", () => notifyModerator(message, approval.id));

      // Suspend until moderator resolves the approval
      // Check the service logs to see how to resolve it over HTTP, e.g.:
      // curl http://localhost:8080/restate/awakeables/sign_.../resolve --json '"approved"'
      return approval.promise;
    }

    return text;
  }
  ```

  ```python Python {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/python-patterns/app/human_in_the_loop.py?collapse_prequel"}  theme={null}
  content_moderator = restate.Service("HumanInTheLoopService")


  @content_moderator.handler()
  async def moderate(ctx: restate.Context, content: Content) -> str | None:
      """Moderate content with optional human review."""

      # Run LLM moderation
      result = await ctx.run_typed(
          "moderate",
          llm_call,  # Use your preferred LLM SDK here
          RunOptions(max_attempts=3),
          messages=f"""You are a content moderation agent.
          Decide if the content violates policy: {content.message}
          Request human review via tools if policy violation is uncertain.""",
          tools=[tool("get_human_review", "Request human review")],
      )

      # Handle human review request
      if result.tool_calls and result.tool_calls[0].function.name == "get_human_review":
          # Create a recoverable approval promise
          approval_id, approval_promise = ctx.awakeable(type_hint=str)

          await ctx.run_typed(
              "notify moderator",
              notify_moderator,
              content=content,
              approval_id=approval_id,
          )

          # Suspend until moderator resolves the approval
          # Check the service logs to see how to resolve it over HTTP, e.g.:
          # curl http://localhost:8080/restate/awakeables/sign_.../resolve --json '"approved"'
          return await approval_promise

      return result.content
  ```
</CodeGroup>

View on GitHub: [TS](https://github.com/restatedev/ai-examples/blob/typescript_patterns/typescript-patterns/src/human-in-the-loop.ts) /
[Python](https://github.com/restatedev/ai-examples/blob/main/python-patterns/app/human_in_the_loop.py)

Check out the SDK documentation for more details on the awakeables and durable promises API ([TS](/develop/ts/external-events) / [Python](/develop/python/external-events)).

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
      <Tabs>
        <Tab title="UI">
          In the UI (`http://localhost:9070`), click on the `moderate` handler of the `CallChainingService` to open the playground and send a default request:

          <img alt="Human approval playground - UI" />
        </Tab>

        <Tab title="curl">
          Or use `curl` to send a request directly to the service:

          <CodeGroup>
            ```shell TypeScript theme={null}
            curl localhost:8080/HumanInTheLoopService/moderate \
            --json '{"message": "Very explicit content that might violate the policy."}'
            ```

            ```shell Python theme={null}
            curl localhost:8080/HumanInTheLoopService/moderate \
            --json '{"message": "Very explicit content that might violate the policy."}'
            ```
          </CodeGroup>
        </Tab>
      </Tabs>

      This will block on the human approval step, so you will not see a response yet.

      You can see in the Invocations Tab of the UI how the workflow suspends after waiting for a minute:

      <img alt="Human approval suspended journal" />
    </Step>

    <Step title="Resolve the approval">
      Check the service logs to find the curl command to resolve the approval. It will look like this:

      ```shell theme={null}
      curl http://localhost:8080/restate/awakeables/{awakeable_id}/resolve --json '"approved"'
      ```

      Replace `{awakeable_id}` with the actual ID from the logs.
    </Step>

    <Step title="Check the Restate UI">
      You can see in the Invocations Tab of the UI how the workflow suspends during the human approval step and resumes once the promise is resolved.

      <img alt="Human approval - journal" />
    </Step>
  </Steps>
</Accordion>