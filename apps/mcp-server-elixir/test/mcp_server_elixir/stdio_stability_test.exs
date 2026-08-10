defmodule McpServerElixir.StdioStabilityTest do
  @moduledoc """
  Regression tests for #229's fourth ask — the stdio connection drops and the
  client's tools "vanish and come back with no config change".

  Two mechanisms, both of which corrupt or kill the one stream the client has.

  1. **Logger wrote to stdout.** stdout IS the JSON-RPC channel; anything else
     written there is protocol garbage. `Server.init/1` says "Log to stderr (MCP
     protocol uses stdout for JSON-RPC)" but only set the encoding on
     `:standard_error` — it never moved Logger's device, and Elixir's default
     handler logs to `:standard_io`. Every `Logger` line — including the
     intermittent WebSocket reconnect logs, which fire with no config change —
     landed mid-stream.

  2. **An exception while handling a request killed the server.** Every handler
     starts with `Map.fetch!(args, "<required>")`, so a client that omits a
     required argument — routine for an LLM-driven caller — raised a `KeyError`
     straight out of `handle_info/2`, taking the GenServer and the connection
     with it. The same is true one level up, before any tool is reached:
     `"arguments"` arriving as a JSON string or a list raises a `BadMapError` in
     `call_tool/3`'s argument shaping, and a result Jason cannot encode raises
     on the *success* path in `send_response/1`. The guard therefore has to sit
     at the seam, around the whole of `process_request/2`.

  A bad request must come back as a JSON-RPC error, and nothing but JSON-RPC
  may ever reach stdout.
  """
  use ExUnit.Case, async: true

  import ExUnit.CaptureIO

  alias McpServerElixir.Protocol.McpTools
  alias McpServerElixir.Server

  defmodule StubBackend do
    @moduledoc false
    def query_events(_params), do: {:ok, %{"events" => [], "count" => 0}}
    def reconstruct_state(_entity_id, _as_of), do: {:ok, %{"current_state" => %{}}}
  end

  defmodule GarbageBodyBackend do
    @moduledoc false
    # A gateway that answers 200 with a non-JSON-object body. The handler then
    # calls Map.get/3 on a binary and raises mid-flight.
    def query_events(_params), do: {:ok, "not a map"}
  end

  defp state do
    %{
      backend: StubBackend,
      read_only: false,
      control_plane_enabled: false,
      system_admin: false
    }
  end

  describe "stdout carries JSON-RPC only" do
    test "Logger's default handler writes to stderr, not the protocol stream" do
      assert {:ok, %{config: %{type: type}}} = :logger.get_handler_config(:default)

      assert type == :standard_error,
             "Logger is writing to #{inspect(type)}; on a stdio MCP server every " <>
               "log line would land in the JSON-RPC stream"
    end

    test "no environment config re-points the default handler at stdout" do
      # The assertion above only resolves MIX_ENV=test. A `config :logger,
      # :default_handler` line added to prod.exs would ship green while breaking
      # the released server, so check the env files statically too.
      for file <- Path.wildcard("config/*.exs"), Path.basename(file) != "config.exs" do
        source = File.read!(file)

        refute source =~ ":default_handler",
               "#{file} configures :default_handler; the stderr setting in " <>
                 "config/config.exs must stay the only one, or a release can " <>
                 "put log lines back on the JSON-RPC stream"
      end
    end
  end

  describe "a raising tool handler is reported, not fatal" do
    test "call_tool returns an error when a required argument is missing" do
      assert {:error, message} = McpTools.call_tool("reconstruct_state", %{}, state())

      # The reason names the missing key; Server.error_message/2 adds the tool.
      assert message =~ "entity_id"
    end

    test "call_tool survives a handler that raises mid-flight" do
      st = %{state() | backend: GarbageBodyBackend}

      assert {:error, message} =
               McpTools.call_tool("event_timeline", %{"entity_id" => "e-1"}, st)

      assert message =~ "map"
    end
  end

  describe "the server GenServer survives a raising tool call" do
    test "a tools/call with missing arguments answers -32603 instead of crashing" do
      line =
        Jason.encode!(%{
          "jsonrpc" => "2.0",
          "id" => 7,
          "method" => "tools/call",
          "params" => %{"name" => "reconstruct_state", "arguments" => %{}}
        })

      st = state()

      output =
        capture_io(fn ->
          assert {:noreply, ^st} = Server.handle_info({:stdin_line, line}, st)
        end)

      assert {:ok, response} = Jason.decode(output)
      assert response["id"] == 7
      assert response["error"]["code"] == -32_603
      assert response["error"]["message"] =~ "reconstruct_state"
    end
  end

  describe "the seam survives client-shaped input the tool layer never sees" do
    # `McpTools.call_tool/3` shapes arguments (`Map.get(args, "format")`,
    # `Map.delete(args, "format")`) BEFORE its own rescue can run, and
    # `process_request/2` reads `params["name"]` before that. A rescue inside
    # call_tool therefore guards nothing against a client that sends the wrong
    # JSON *type*. Real MCP/LLM clients routinely send `arguments` as a JSON
    # string. The barrier has to sit at `handle_info/2`, or one such request
    # kills the GenServer — and four in five seconds trip the supervisor's
    # default max_restarts and take the whole application down.

    defp call_line(params, id) do
      Jason.encode!(%{
        "jsonrpc" => "2.0",
        "id" => id,
        "method" => "tools/call",
        "params" => params
      })
    end

    defp assert_survives(line, id) do
      st = state()

      output =
        capture_io(fn ->
          assert {:noreply, ^st} = Server.handle_info({:stdin_line, line}, st)
        end)

      assert {:ok, response} = Jason.decode(output)
      assert response["id"] == id
      assert response["error"]["code"] == -32_603
      response
    end

    test "`arguments` sent as a JSON string answers -32603 instead of killing the session" do
      line =
        call_line(
          %{"name" => "reconstruct_state", "arguments" => ~s({"entity_id":"e1"})},
          11
        )

      response = assert_survives(line, 11)
      assert response["error"]["message"] =~ "reconstruct_state"
    end

    test "`arguments` sent as a list answers -32603 instead of killing the session" do
      line = call_line(%{"name" => "reconstruct_state", "arguments" => ["e1"]}, 12)

      response = assert_survives(line, 12)
      assert response["error"]["message"] =~ "reconstruct_state"
    end

    test "`params` sent as a string answers -32603 instead of killing the session" do
      line =
        Jason.encode!(%{
          "jsonrpc" => "2.0",
          "id" => 13,
          "method" => "tools/call",
          "params" => "reconstruct_state"
        })

      assert_survives(line, 13)
    end

    test "a notification whose handling raises is swallowed, not fatal" do
      # No id => no response is owed, but the process must still live.
      line =
        Jason.encode!(%{
          "jsonrpc" => "2.0",
          "method" => "tools/call",
          "params" => %{"name" => "reconstruct_state", "arguments" => "oops"}
        })

      st = state()

      output =
        capture_io(fn ->
          assert {:noreply, ^st} = Server.handle_info({:stdin_line, line}, st)
        end)

      assert output == ""
    end
  end

  describe "a result Jason cannot encode degrades to an error" do
    # `send_response/1` called `Jason.encode!` with no protection, inside the
    # GenServer. No shipping handler builds a non-encodable result today, so
    # this is a barrier against the next one that does: the success path must
    # not be a second way to kill the stdio session.

    test "send_response emits -32603 rather than raising" do
      response = %{
        jsonrpc: "2.0",
        id: 21,
        result: %{content: [%{type: "text", text: self()}]}
      }

      output = capture_io(fn -> assert :ok = Server.send_response(response) end)

      assert {:ok, decoded} = Jason.decode(output)
      assert decoded["id"] == 21
      assert decoded["error"]["code"] == -32_603
      refute Map.has_key?(decoded, "result")
    end
  end

  describe "explain_entity with no events" do
    defmodule NoEventsBackend do
      @moduledoc false
      # State reconstructs (Core can seed from a snapshot) while the entity's
      # events are gone — archived or compacted away. `List.first([])` is nil,
      # and `Map.get(nil, "timestamp")` raised a BadMapError here.
      def reconstruct_state(_entity_id, _as_of),
        do: {:ok, %{"current_state" => %{"status" => "active"}, "last_updated" => "2026-01-01"}}

      def query_events(_params), do: {:ok, %{"events" => [], "count" => 0}}
    end

    test "reports the entity instead of blowing up on an empty event list" do
      st = %{state() | backend: NoEventsBackend}

      assert {:ok, %{content: content}} =
               McpTools.call_tool("explain_entity", %{"entity_id" => "e-1"}, st)

      assert Enum.any?(content, &(&1.text =~ "e-1"))
    end
  end
end
