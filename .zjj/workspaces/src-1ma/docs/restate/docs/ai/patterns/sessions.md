# Durable Sessions

Source: https://docs.restate.dev/ai/patterns/sessions

Build persistent chat sessions that survive interruptions and maintain conversation state.

Build persistent, stateful chat sessions that handle long-running conversations across multiple interactions and users.

A user might start a conversation now, respond hours later, and return again after a few days. Multiple users may be having separate conversations going on, and a single conversation may be open in multiple browser windows.

This guide helps you with implementing persistent chat sessions with Restate.

## Virtual Objects

To implement stateful entities like chat sessions, or stateful agents, Restate provides the service type **Virtual Objects**.

Each Virtual Object instance maintains isolated state and is identified by a unique key.

Here is an example of a Virtual Object that represents chat sessions:

<img alt="Objects" />

<CodeGroup>
  ```ts TypeScript {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/typescript-patterns/src/chat.ts#here"}  theme={null}
  export default restate.object({
    name: "Chat",
    handlers: {
      message: restate.createObjectHandler(
        { input: zodPrompt(examplePrompt) },
        async (ctx: ObjectContext, { message }: { message: string }) => {
          const messages = (await ctx.get<Array<ModelMessage>>("memory")) ?? [];
          messages.push({ role: "user", content: message });

          // Use your preferred LLM SDK here
          const result = await ctx.run("LLM call", async () => llmCall(messages));

          messages.push({ role: "assistant", content: result.text });
          ctx.set("memory", messages);

          return result.text;
        },
      ),
      getHistory: restate.createObjectSharedHandler(
        async (ctx: restate.ObjectSharedContext) =>
          ctx.get<Array<ModelMessage>>("memory"),
      ),
    },
  });
  ```

  ```python Python {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/python-patterns/app/chat.py?collapse_prequel"}  theme={null}
  chat = restate.VirtualObject("Chat")


  @chat.handler()
  async def message(ctx: restate.ObjectContext, msg: ChatMessage) -> str | None:
      """A long-lived stateful chat session that allows for ongoing conversation."""

      # Retrieve conversation memory from Restate
      messages = await ctx.get("memory", type_hint=list[dict]) or []
      messages.append({"role": "user", "content": msg.message})

      result = await ctx.run_typed(
          "LLM call",
          llm_call,  # Use your preferred LLM SDK here
          RunOptions(max_attempts=3),
          messages=messages,
      )

      # Update conversation memory in Restate
      messages.append({"role": "assistant", "content": result.content})
      ctx.set("memory", messages)

      return result.content


  @chat.handler(kind="shared")
  async def get_history(ctx: restate.ObjectSharedContext):
      return await ctx.get("memory", type_hint=list[dict]) or []
  ```
</CodeGroup>

View on GitHub: [TS](https://github.com/restatedev/ai-examples/blob/main/typescript-patterns/src/chat.ts) /
[Python](https://github.com/restatedev/ai-examples/blob/main/python-patterns/app/chat.py)

This example stores the chat messages in Restate. You can also store any other K/V state, like user preferences.

Virtual Objects provide:

* **Durable state**: Conversation history or any other K/V state (e.g. user preferences) that persists across failures and restarts
* **Session isolation**: Each chat gets isolated state with automatic concurrency control ([see below](/ai/patterns/sessions#built-in-concurrency-control)
* Works with **any LLM SDK** (Vercel AI, LangChain, LiteLLM, etc.) and **any programming language** supported by Restate (TypeScript, Python, Go, etc.).

The UI lets you query the state of each chat session:

<img alt="Chat session state - UI" />

<Tip>
  This pattern is complementary to AI memory solutions like mem0 or graffiti. You can use Virtual Objects to enforce session concurrency and queueing while storing the agent's memory in specialized memory systems.
</Tip>

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

    <Step title="Send messages to a chat session">
      <Tabs>
        <Tab title="UI">
          In the UI (`http://localhost:9070`), click on the `message` handler of the `Chat` service to open the playground.
          Enter a key for the chat session (e.g., `session123`) and send messages to start a conversation.

          <img alt="Chat playground" />
        </Tab>

        <Tab title="curl">
          Send a request to the service:

          <CodeGroup>
            ```shell TypeScript theme={null}
            curl localhost:8080/Chat/session123/message \
              --json '{"message": "Write a poem about Durable Execution?"}'
            ```

            ```shell Python theme={null}
            curl localhost:8080/Chat/session123/message \
              --json '{"message": "Write a poem about Durable Execution?"}'
            ```
          </CodeGroup>
        </Tab>
      </Tabs>

      The session state (conversation history) is automatically persisted and maintained across calls.
      Send additional messages with the same session ID to see how the conversation context is preserved. For example, ask to shorten the poem.
    </Step>

    <Step title="Check the Restate UI">
      In the State Tab, you can view what is stored in Restate for each chat session:

      <img alt="Chat session state - UI" />
    </Step>
  </Steps>
</Accordion>

## Built-in concurrency control

Restate’s Virtual Objects have built-in queuing and consistency guarantees per object key. Handlers either have read-write access (`ObjectContext`) or read-only access (shared object context).

* Only one handler with write access can run at a time per object key to prevent concurrent/lost writes or race conditions (for example `message()`).
* Handlers with read-only access can run concurrently to the write-access handlers (for example `getHistory()`).

<img alt="Queue" />

**Seeing concurrency control in action:**

In the chat service, the `message` handler is an exclusive handler, while the `getHistory` handler is a shared handler.

Let's send some messages to a chat session:

```bash theme={null}
curl localhost:8080/Chat/session123/message/send --json '{"message": "make a poem about durable execution"}' &
curl localhost:8080/Chat/session456/message/send --json '{"message": "what are the benefits of durable execution?"}' &
curl localhost:8080/Chat/session789/message/send --json '{"message": "how does workflow orchestration work?"}' &
curl localhost:8080/Chat/session123/message/send --json '{"message": "can you make it rhyme better?"}' &
curl localhost:8080/Chat/session456/message/send --json '{"message": "what about fault tolerance in distributed systems?"}' &
curl localhost:8080/Chat/session789/message/send --json '{"message": "give me a practical example"}' &
curl localhost:8080/Chat/session101/message/send --json '{"message": "explain event sourcing in simple terms"}' &
curl localhost:8080/Chat/session202/message/send --json '{"message": "what is the difference between async and sync processing?"}'
```

The UI shows how Restate queues the requests per session to ensure consistency. Only one chat agent is running per session ID:

<Frame>
  <img alt="Conversation State Management" />
</Frame>

## Retrieving state

The state you store in Virtual Objects lives forever. If you want to resume a session, this means simply sending a new message to the same virtual object.

To retrieve the state, we can add a handler which reads the state. Have a look at the `getHistory`/`get_history` handler in the example above.

Call the handler to get the history:

<CodeGroup>
  ```shell TypeScript theme={null}
  curl localhost:8080/Chat/user123/getHistory
  ```

  ```shell Python theme={null}
  curl localhost:8080/Chat/user123/get_history
  ```
</CodeGroup>

This is a [shared handler](/foundations/handlers#handler-behavior), meaning it can only read state (not write). This allows it to run concurrently with the `message` handler.