# AI Agent Quickstart

Source: https://docs.restate.dev/ai-quickstart

Build and run your first AI agent with Restate and popular AI SDKs

This guide takes you through building your first AI agent with Restate and popular AI SDKs.

We will run a simple weather agent that can answer questions about the weather using durable execution to ensure reliability.

<img alt="AI Agent Quickstart" />

Select your AI SDK:

<Tabs>
  <Tab title="TypeScript + Vercel AI" icon={"/img/languages/typescript.svg"}>
    <Info>
      **Prerequisites**:

      * [Node.js](https://nodejs.org/en/) >= v20
      * OpenAI API key (get one at [OpenAI](https://platform.openai.com/))
    </Info>

    <Steps>
      <Step title="Install Restate Server & CLI">
        Restate is a single self-contained binary. No external dependencies needed.

        <Tabs>
          <Tab title="Homebrew">
            ```shell theme={null}
            brew install restatedev/tap/restate-server restatedev/tap/restate
            ```

            Start the server:

            ```shell theme={null}
            restate-server
            ```
          </Tab>

          <Tab title="Download binaries">
            Download prebuilt binaries from the [releases page](https://github.com/restatedev/restate/releases/latest):

            <CodeGroup>
              ```shell MacOS-x64 theme={null}
              BIN=/usr/local/bin && RESTATE_PLATFORM=x86_64-apple-darwin && \
              curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
              tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
              tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
              chmod +x restate restate-server && \
              sudo mv restate $BIN && \
              sudo mv restate-server $BIN
              ```

              ```shell MacOS-arm64 theme={null}
              BIN=/usr/local/bin && RESTATE_PLATFORM=aarch64-apple-darwin && \
              curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
              tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
              tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
              chmod +x restate restate-server && \
              sudo mv restate $BIN && \
              sudo mv restate-server $BIN
              ```

              ```shell Linux-x64 theme={null}
              BIN=$HOME/.local/bin && RESTATE_PLATFORM=x86_64-unknown-linux-musl && \
              curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
              tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
              tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
              chmod +x restate restate-server && \
              mv restate $BIN && \
              mv restate-server $BIN
              ```

              ```shell Linux-arm64 theme={null}
              BIN=$HOME/.local/bin && RESTATE_PLATFORM=aarch64-unknown-linux-musl && \
              curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
              tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
              tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
              chmod +x restate restate-server && \
              mv restate $BIN && \
              mv restate-server $BIN
              ```
            </CodeGroup>

            Start the server:

            ```shell theme={null}
            restate-server
            ```
          </Tab>

          <Tab title="npm">
            ```shell theme={null}
            npm install --global @restatedev/restate-server@latest @restatedev/restate@latest
            ```

            Start the server:

            ```shell theme={null}
            restate-server
            ```
          </Tab>

          <Tab title="Docker">
            Run the Restate Server:

            ```shell theme={null}
            docker run --name restate_dev --rm \
            -p 8080:8080 -p 9070:9070 -p 9071:9071 \
            --add-host=host.docker.internal:host-gateway \
            docker.restate.dev/restatedev/restate:latest
            ```

            Run CLI commands:

            ```shell theme={null}
            docker run -it --network=host \
            docker.restate.dev/restatedev/restate-cli:latest \
            invocations ls
            ```

            Replace `invocations ls` with any CLI subcommand.
          </Tab>
        </Tabs>

        You can find the Restate UI running on port 9070 (`http://localhost:9070`) after starting the Restate Server.
      </Step>

      <Step title="Get the AI Agent template">
        Get the weather agent template for the [Vercel AI SDK](https://ai-sdk.dev/docs/foundations/overview) and Restate:

        ```shell theme={null}
        git clone https://github.com/restatedev/ai-examples.git &&
        cd ai-examples/vercel-ai/template &&
        npm install
        ```

        <GitHubLink />
      </Step>

      <Step title="Run the AI Agent service">
        Export your OpenAI key and run the agent:

        ```shell theme={null}
        export OPENAI_API_KEY=your_openai_api_key_here
        npm run dev
        ```

        The weather agent is now listening on port 9080.
      </Step>

      <Step title="Register the service">
        Tell Restate where the service is running (`http://localhost:9080`), so Restate can discover and register the services and handlers behind this endpoint.
        You can do this via the UI (`http://localhost:9070`) or via:

        <CodeGroup>
          ```shell CLI theme={null}
          restate deployments register http://localhost:9080
          ```

          ```shell curl theme={null}
          curl localhost:9070/deployments --json '{"uri": "http://localhost:9080"}'
          ```
        </CodeGroup>

        <Expandable title="Output">
          <CodeGroup>
            ```shell CLI theme={null}
            ❯ SERVICES THAT WILL BE ADDED:
            - agent
            Type: Service
            HANDLER  INPUT                                     OUTPUT
            run      value of content-type 'application/json'  value of content-type 'application/json'


            ✔ Are you sure you want to apply those changes? · yes
            ✅ DEPLOYMENT:
            SERVICE  REV
            agent    1
            ```

            ```shell curl theme={null}
            {
                "id": "dp_17sztQp4gnEC1L0OCFM9aEh",
                "services": [
                    {
                        "name": "Agent",
                        "handlers": [
                            {
                                "name": "run",
                                "ty": "Shared",
                                "input_description": "one of [\"none\", \"value of content-type 'application/json'\"]",
                                "output_description": "value of content-type 'application/json'"
                            }
                        ],
                        "ty": "Service",
                        "deployment_id": "dp_17sztQp4gnEC1L0OCFM9aEh",
                        "revision": 1,
                        "public": true,
                        "idempotency_retention": "1day"
                    }
                ]
            }
            ```
          </CodeGroup>
        </Expandable>

        If you run Restate with Docker, register `http://host.docker.internal:9080` instead of `http://localhost:9080`.

        <Accordion title="Restate Cloud">
          When using [Restate Cloud](https://restate.dev/cloud), your service must be accessible over the public internet so Restate can invoke it.
          If you want to develop with a local service, you can expose it using our [tunnel](/deploy/server/cloud/#registering-restate-services-with-your-environment) feature.
        </Accordion>
      </Step>

      <Step title="Send weather requests to the AI Agent">
        Invoke the agent via the Restate UI playground: go to `http://localhost:9070`, click on your service and then on playground.

        <Frame>
          <img alt="Restate UI Playground" />
        </Frame>

        Or invoke via `curl`:

        ```shell theme={null}
        curl localhost:8080/agent/run --json '"What is the weather in Detroit?"'
        ```

        Output: `The weather in Detroit is currently 17°C with misty conditions.`.
      </Step>

      <Step title="Congratulations, you just ran a Durable AI Agent!">
        The agent you just invoked uses Durable Execution to make agents resilient to failures. Restate persisted all LLM calls and tool execution steps, so if anything fails, the agent can resume exactly where it left off.

        We did this by using Restate's `durableCalls` middleware to persist LLM responses and using [Restate Context actions](/foundations/actions) (e.g. `ctx.run`) to make the tool executions resilient:

        ```ts expandable {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/vercel-ai/template/src/app.ts?collapse_imports"}  theme={null}
        async function weatherAgent(restate: restate.Context, prompt: string) {
          // The durableCalls middleware persists each LLM response in Restate,
          // so they can be restored on retries without re-calling the LLM
          const model = wrapLanguageModel({
            model: openai("gpt-4o"),
            middleware: durableCalls(restate, { maxRetryAttempts: 3 }),
          });

          const { text } = await generateText({
            model,
            system: "You are a helpful agent that provides weather updates.",
            prompt,
            tools: {
              getWeather: tool({
                description: "Get the current weather for a given city.",
                inputSchema: z.object({ city: z.string() }),
                execute: async ({ city }) => {
                  // call tool wrapped as Restate durable step
                  return await restate.run("get weather", () => fetchWeather(city));
                },
              }),
            },
            stopWhen: [stepCountIs(5)],
            providerOptions: { openai: { parallelToolCalls: false } },
          });

          return text;
        }

        // create a Restate Service as the callable entrypoint
        // for our durable agent function
        const agent = restate.service({
          name: "agent",
          handlers: {
            run: async (ctx: restate.Context, prompt: string) => {
              return weatherAgent(ctx, prompt);
            },
          },
        });

        // Serve the entry-point via an HTTP/2 server
        restate.serve({
          services: [agent],
        });
        ```

        <GitHubLink />

        The Invocations tab of the Restate UI shows us how Restate captured each LLM call and tool step in a journal:

        <Frame>
          <img alt="Restate UI Journal Entries" />
        </Frame>

        <Accordion title="See how a failing tool call is retried">
          Ask about the weather in Denver:

          ```shell theme={null}
          curl localhost:8080/agent/run --json '"What is the weather in Denver?"'
          ```

          You can see in the service logs and in the Restate UI how each LLM call and tool step gets durably executed.
          We can see how the weather tool is currently stuck, because the weather API is down.

          <Frame>
            <img alt="Restate UI Durable Execution" />
          </Frame>

          This was a mimicked failure. To fix the problem, remove the line `failOnDenver` from the `fetchWeather` function in the `utils.ts` file:

          ```ts {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/vercel-ai/template/src/utils/weather.ts#weather"}  theme={null}
          export async function fetchWeather(city: string) {
            failOnDenver(city);
            const output = await fetchWeatherFromAPI(city);
            return parseWeatherResponse(output);
          }
          ```

          Once you restart the service, the agent resumes at the weather tool call and successfully completes the request.
        </Accordion>

        **Next step:**
        Follow the [Tour of Agents](/tour/vercel-ai-agents) to learn how to build agents with Restate and Vercel AI SDK, OpenAI Agents SDK, etc.
      </Step>
    </Steps>
  </Tab>

  <Tab title="Python + OpenAI" icon={"/img/languages/python.svg"}>
    <Info>
      **Prerequisites**:

      * Python >= v3.12
      * [uv](https://docs.astral.sh/uv/getting-started/installation/)
      * OpenAI API key (get one at [OpenAI](https://platform.openai.com/))
    </Info>

    <iframe title="YouTube video player" />

    <Steps>
      <Step title="Install Restate Server & CLI">
        Restate is a single self-contained binary. No external dependencies needed.

        <Tabs>
          <Tab title="Homebrew">
            ```shell theme={null}
            brew install restatedev/tap/restate-server restatedev/tap/restate
            ```

            Start the server:

            ```shell theme={null}
            restate-server
            ```
          </Tab>

          <Tab title="Download binaries">
            Download prebuilt binaries from the [releases page](https://github.com/restatedev/restate/releases/latest):

            <CodeGroup>
              ```shell MacOS-x64 theme={null}
              BIN=/usr/local/bin && RESTATE_PLATFORM=x86_64-apple-darwin && \
              curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
              tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
              tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
              chmod +x restate restate-server && \
              sudo mv restate $BIN && \
              sudo mv restate-server $BIN
              ```

              ```shell MacOS-arm64 theme={null}
              BIN=/usr/local/bin && RESTATE_PLATFORM=aarch64-apple-darwin && \
              curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
              tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
              tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
              chmod +x restate restate-server && \
              sudo mv restate $BIN && \
              sudo mv restate-server $BIN
              ```

              ```shell Linux-x64 theme={null}
              BIN=$HOME/.local/bin && RESTATE_PLATFORM=x86_64-unknown-linux-musl && \
              curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
              tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
              tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
              chmod +x restate restate-server && \
              mv restate $BIN && \
              mv restate-server $BIN
              ```

              ```shell Linux-arm64 theme={null}
              BIN=$HOME/.local/bin && RESTATE_PLATFORM=aarch64-unknown-linux-musl && \
              curl -L --remote-name-all https://restate.gateway.scarf.sh/latest/restate-{server,cli}-$RESTATE_PLATFORM.tar.xz && \
              tar -xvf restate-server-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-server-$RESTATE_PLATFORM/restate-server && \
              tar -xvf restate-cli-$RESTATE_PLATFORM.tar.xz --strip-components=1 restate-cli-$RESTATE_PLATFORM/restate && \
              chmod +x restate restate-server && \
              mv restate $BIN && \
              mv restate-server $BIN
              ```
            </CodeGroup>

            Start the server:

            ```shell theme={null}
            restate-server
            ```
          </Tab>

          <Tab title="npm">
            ```shell theme={null}
            npm install --global @restatedev/restate-server@latest @restatedev/restate@latest
            ```

            Start the server:

            ```shell theme={null}
            restate-server
            ```
          </Tab>

          <Tab title="Docker">
            Run the Restate Server:

            ```shell theme={null}
            docker run --name restate_dev --rm \
            -p 8080:8080 -p 9070:9070 -p 9071:9071 \
            --add-host=host.docker.internal:host-gateway \
            docker.restate.dev/restatedev/restate:latest
            ```

            Run CLI commands:

            ```shell theme={null}
            docker run -it --network=host \
            docker.restate.dev/restatedev/restate-cli:latest \
            invocations ls
            ```

            Replace `invocations ls` with any CLI subcommand.
          </Tab>
        </Tabs>

        You can find the Restate UI running on port 9070 (`http://localhost:9070`) after starting the Restate Server.
      </Step>

      <Step title="Get the AI Agent template">
        Get the weather agent template for the [OpenAI Agents SDK](https://openai.github.io/openai-agents-python/) and Restate:

        ```shell theme={null}
        git clone https://github.com/restatedev/ai-examples.git &&
        cd ai-examples/openai-agents/template
        ```

        <GitHubLink />
      </Step>

      <Step title="Run the AI Agent service">
        Export your OpenAI key and run the agent:

        ```shell theme={null}
        export OPENAI_API_KEY=your_openai_api_key_here
        uv run .
        ```

        The weather agent is now listening on port 9080.
      </Step>

      <Step title="Register the agent service">
        Tell Restate where the service is running (`http://localhost:9080`), so Restate can discover and register the services and handlers behind this endpoint.
        You can do this via the UI (`http://localhost:9070`) or via:

        <CodeGroup>
          ```shell CLI theme={null}
          restate deployments register http://localhost:9080
          ```

          ```shell curl theme={null}
          curl localhost:9070/deployments --json '{"uri": "http://localhost:9080"}'
          ```
        </CodeGroup>

        <Expandable title="Output">
          <CodeGroup>
            ```shell CLI theme={null}
            ❯ SERVICES THAT WILL BE ADDED:
            - Agent
            Type: Service
            HANDLER  INPUT                                     OUTPUT
            run      value of content-type 'application/json'  value of content-type 'application/json'


            ✔ Are you sure you want to apply those changes? · yes
            ✅ DEPLOYMENT:
            SERVICE  REV
            Agent    1
            ```

            ```shell curl theme={null}
            {
                "id": "dp_17sztQp4gnEC1L0OCFM9aEh",
                "services": [
                    {
                        "name": "Agent",
                        "handlers": [
                            {
                                "name": "run",
                                "ty": "Shared",
                                "input_description": "one of [\"none\", \"value of content-type 'application/json'\"]",
                                "output_description": "value of content-type 'application/json'"
                            }
                        ],
                        "ty": "Service",
                        "deployment_id": "dp_17sztQp4gnEC1L0OCFM9aEh",
                        "revision": 1,
                        "public": true,
                        "idempotency_retention": "1day"
                    }
                ]
            }
            ```
          </CodeGroup>
        </Expandable>

        If you run Restate with Docker, register `http://host.docker.internal:9080` instead of `http://localhost:9080`.

        <Accordion title="Restate Cloud">
          When using [Restate Cloud](https://restate.dev/cloud), your service must be accessible over the public internet so Restate can invoke it.
          If you want to develop with a local service, you can expose it using our [tunnel](/deploy/server/cloud/#registering-restate-services-with-your-environment) feature.
        </Accordion>
      </Step>

      <Step title="Send weather requests to the AI Agent">
        Invoke the agent via the Restate UI playground: go to `http://localhost:9070`, click on your service and then on playground.

        <Frame>
          <img alt="Restate UI Playground" />
        </Frame>

        Or invoke via `curl`:

        ```shell theme={null}
        curl localhost:8080/agent/run --json '{
                "message": "What is the weather in Detroit?"
            }'
        ```

        Output: `The weather in Detroit is currently 17°C with misty conditions.`
      </Step>

      <Step title="Congratulations, you just ran a Durable AI Agent!">
        The agent you just invoked uses Durable Execution to make agents resilient to failures. Restate persisted all LLM calls and tool execution steps, so if anything fails, the agent can resume exactly where it left off.

        We did this by using Restate's `DurableRunner` to persist LLM responses, and by using [Restate Context actions](/foundations/actions) (e.g. `restate_context().run_typed`) to make the tool executions resilient:

        ```python expandable {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/openai-agents/template/agent.py?collapse_imports"}  theme={null}
        @durable_function_tool
        async def get_weather(req: WeatherRequest) -> WeatherResponse:
            """Get the current weather for a given city."""
            # Do durable steps using the Restate context
            return await restate_context().run_typed(
                "Get weather", fetch_weather, city=req.city
            )


        weather_agent = Agent(
            name="WeatherAgent",
            instructions="You are a helpful agent that provides weather updates.",
            tools=[get_weather],
        )


        agent_service = restate.Service("agent")


        @agent_service.handler()
        async def run(_ctx: restate.Context, req: WeatherPrompt) -> str:
            # Runner that persists the agent execution for recoverability
            result = await DurableRunner.run(weather_agent, req.message)
            return result.final_output
        ```

        <GitHubLink />

        The Invocations tab of the Restate UI shows us how Restate captured each LLM call and tool step in a journal:

        <Frame>
          <img alt="Restate UI Journal Entries" />
        </Frame>

        <Accordion title="See how a failing tool call is retried">
          Ask about the weather in Denver:

          ```shell theme={null}
          curl localhost:8080/agent/run --json '{
                  "message": "What is the weather in Denvewr?"
              }'
          ```

          You can see in the Restate UI how each LLM call and tool step gets durably executed.
          We can see how the weather tool is currently stuck, because the weather API is down.

          <Frame>
            <img alt="Restate UI Durable Execution" />
          </Frame>

          This was a mimicked failure. To fix the problem, remove the line `fail_on_denver` from the `fetch_weather` function in the `utils.py` file:

          ```python {"CODE_LOAD::https://raw.githubusercontent.com/restatedev/ai-examples/refs/heads/main/openai-agents/template/utils/utils.py#weather"}  theme={null}
          async def fetch_weather(city: str) -> WeatherResponse:
              fail_on_denver(city)
              weather_data = await call_weather_api(city)
              return parse_weather_data(weather_data)
          ```

          Once you restart the service, the agent resumes at the weather tool call and successfully completes the request.
        </Accordion>
      </Step>
    </Steps>
  </Tab>
</Tabs>

<Tip>
  [Once you are done with the quickstart, check out how to write Restate apps with AI assistants.](/develop/ai-assistant)
</Tip>