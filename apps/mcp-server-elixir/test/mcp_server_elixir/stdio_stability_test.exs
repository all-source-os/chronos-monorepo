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

  describe "the request-shape space" do
    # One property over the whole space of client-controlled JSON-RPC shapes,
    # rather than a one-off per shape. Two invariants, both of which #229 broke:
    #
    #   1. `handle_info/2` returns `{:noreply, state}` — the GenServer, and with
    #      it the client's only stream, survives every shape.
    #   2. stdout carries exactly the JSON-RPC frames the protocol owes and
    #      nothing else: one correlated line per request that carries an id,
    #      not a single byte for a notification.
    #
    # A shape only needs adding here; the invariants come for free.

    defp deep_map(0), do: %{"leaf" => true}
    defp deep_map(n), do: %{"a" => deep_map(n - 1)}

    defp line(map), do: Jason.encode!(map)

    defp call(name, arguments, extra \\ %{}) do
      Map.merge(
        %{
          "jsonrpc" => "2.0",
          "method" => "tools/call",
          "params" => %{"name" => name, "arguments" => arguments}
        },
        extra
      )
    end

    # {label, line, expected id echoed back}
    defp request_shapes do
      [
        {"arguments as a JSON string", line(call("reconstruct_state", "e1", %{"id" => 1})), 1},
        {"arguments as a list", line(call("reconstruct_state", ["e1"], %{"id" => 2})), 2},
        {"arguments as a number", line(call("reconstruct_state", 5, %{"id" => 3})), 3},
        {"arguments as null", line(call("reconstruct_state", nil, %{"id" => 4})), 4},
        {"arguments as a boolean", line(call("reconstruct_state", true, %{"id" => 5})), 5},
        {"arguments 200 levels deep", line(call("reconstruct_state", deep_map(200), %{"id" => 6})),
         6},
        {"a 64KB argument value",
         line(
           call("reconstruct_state", %{"entity_id" => String.duplicate("x", 64 * 1024)}, %{
             "id" => 7
           })
         ), 7},
        {"params as a string",
         line(%{
           "jsonrpc" => "2.0",
           "id" => 8,
           "method" => "tools/call",
           "params" => "reconstruct_state"
         }), 8},
        {"params as a list",
         line(%{"jsonrpc" => "2.0", "id" => 9, "method" => "tools/call", "params" => [1, 2]}), 9},
        {"params as null",
         line(%{"jsonrpc" => "2.0", "id" => 10, "method" => "tools/call", "params" => nil}), 10},
        {"params absent", line(%{"jsonrpc" => "2.0", "id" => 11, "method" => "tools/call"}), 11},
        {"tool name absent",
         line(%{
           "jsonrpc" => "2.0",
           "id" => 12,
           "method" => "tools/call",
           "params" => %{"arguments" => %{}}
         }), 12},
        {"tool name as a number",
         line(%{
           "jsonrpc" => "2.0",
           "id" => 13,
           "method" => "tools/call",
           "params" => %{"name" => 3, "arguments" => %{}}
         }), 13},
        {"tool name as an object",
         line(%{
           "jsonrpc" => "2.0",
           "id" => 14,
           "method" => "tools/call",
           "params" => %{"name" => %{"a" => 1}, "arguments" => %{}}
         }), 14},
        {"unknown tool name", line(call("no_such_tool", %{}, %{"id" => 15})), 15},
        {"required argument missing", line(call("reconstruct_state", %{}, %{"id" => 16})), 16},
        {"format as a list",
         line(
           call("reconstruct_state", %{"entity_id" => "e1", "format" => ["json"]}, %{"id" => 17})
         ), 17},
        {"method as a number", line(%{"jsonrpc" => "2.0", "id" => 18, "method" => 42}), 18},
        {"method as null", line(%{"jsonrpc" => "2.0", "id" => 19, "method" => nil}), 19},
        {"method as an object", line(%{"jsonrpc" => "2.0", "id" => 20, "method" => %{"a" => 1}}),
         20},
        {"unknown method", line(%{"jsonrpc" => "2.0", "id" => 21, "method" => "frobnicate"}), 21},
        {"id as a string", line(call("no_such_tool", %{}, %{"id" => "abc"})), "abc"},
        {"id as a float", line(call("no_such_tool", %{}, %{"id" => 1.5})), 1.5},
        {"id as an object", line(call("no_such_tool", %{}, %{"id" => %{"a" => 1}})), %{"a" => 1}},
        {"id as a list", line(call("no_such_tool", %{}, %{"id" => [1]})), [1]},
        {"initialize", line(%{"jsonrpc" => "2.0", "id" => 22, "method" => "initialize"}), 22},
        {"tools/list", line(%{"jsonrpc" => "2.0", "id" => 23, "method" => "tools/list"}), 23}
      ]
    end

    # Notifications — no id at all, or the null id MCP forbids. JSON-RPC 2.0:
    # "The Server MUST NOT reply to a Notification."
    defp notification_shapes do
      [
        # The live one. MCP clients emit these routinely — every cancelled tool
        # call, every progress tick — and each used to draw a -32601 with
        # `"id": null` onto the JSON-RPC stream, which the MCP SDK surfaces as
        # "a response for an unknown message ID" (#229).
        {"notifications/cancelled",
         line(%{
           "jsonrpc" => "2.0",
           "method" => "notifications/cancelled",
           "params" => %{"requestId" => 1, "reason" => "user cancelled"}
         })},
        {"notifications/progress",
         line(%{
           "jsonrpc" => "2.0",
           "method" => "notifications/progress",
           "params" => %{"progressToken" => "t", "progress" => 1}
         })},
        {"notifications/roots/list_changed",
         line(%{"jsonrpc" => "2.0", "method" => "notifications/roots/list_changed"})},
        {"notifications/initialized",
         line(%{"jsonrpc" => "2.0", "method" => "notifications/initialized"})},
        {"tools/list as a notification", line(%{"jsonrpc" => "2.0", "method" => "tools/list"})},
        {"initialize as a notification",
         line(%{"jsonrpc" => "2.0", "method" => "initialize", "params" => %{}})},
        {"a tools/call notification that succeeds",
         line(call("reconstruct_state", %{"entity_id" => "e1"}))},
        {"a tools/call notification for an unknown tool", line(call("no_such_tool", %{}))},
        {"a tools/call notification that raises", line(call("reconstruct_state", "oops"))},
        {"an explicit null id", line(call("no_such_tool", %{}, %{"id" => nil}))}
      ]
    end

    defp unparseable_shapes do
      [
        {"not JSON at all", "not json at all"},
        {"a JSON array (batch)", ~s([{"jsonrpc":"2.0","id":1,"method":"tools/list"}])},
        {"a top-level string", ~s("hello")},
        {"a top-level number", "42"},
        {"an empty object", "{}"},
        {"no method field", ~s({"jsonrpc":"2.0","id":1})},
        {"the wrong jsonrpc version", ~s({"jsonrpc":"1.0","id":1,"method":"tools/list"})},
        {"a truncated object", ~s({"jsonrpc":"2.0","id":1,"method":)},
        {"a lone surrogate escape", ~s({"jsonrpc":"2.0","id":1,"method":"\\ud800"})}
      ]
    end

    # Drive the real seam and hand back whatever reached the protocol stream.
    defp drive(line) do
      st = state()

      output =
        capture_io(fn ->
          assert {:noreply, ^st} = Server.handle_info({:stdin_line, line}, st)
        end)

      output
    end

    defp assert_one_json_rpc_frame(output, label) do
      refute output == "",
             "#{label}: answered nothing at all. The client is holding a correlation id it " <>
               "will never see resolved — from its side the tool call hangs until the session " <>
               "is torn down"

      assert String.ends_with?(output, "\n"),
             "#{label}: stdout frame is not newline-terminated — MCP reads stdout as JSON lines"

      lines = String.split(output, "\n", trim: true)

      assert length(lines) == 1,
             "#{label}: wrote #{length(lines)} lines to the JSON-RPC stream, expected exactly 1"

      assert {:ok, frame} = Jason.decode(output), "#{label}: stdout is not valid JSON: #{output}"
      assert frame["jsonrpc"] == "2.0", "#{label}: frame is not JSON-RPC 2.0"

      assert Map.has_key?(frame, "result") != Map.has_key?(frame, "error"),
             "#{label}: frame must carry exactly one of result/error"

      frame
    end

    test "every request shape answers exactly one correlated JSON-RPC frame, and lives" do
      for {label, line, expected_id} <- request_shapes() do
        frame = line |> drive() |> assert_one_json_rpc_frame(label)

        assert frame["id"] == expected_id,
               "#{label}: answered id #{inspect(frame["id"])}, expected #{inspect(expected_id)} — " <>
                 "an uncorrelated id is a response the client cannot match to its request"
      end
    end

    test "no notification shape puts a single byte on the JSON-RPC stream" do
      for {label, line} <- notification_shapes() do
        assert drive(line) == "",
               "#{label}: wrote a response to a notification. JSON-RPC 2.0 forbids it and the " <>
                 "frame carries `\"id\": null`, which no client can correlate — the MCP SDK " <>
                 "reports it as a response for an unknown message ID (#229)"
      end
    end

    test "every unparseable shape answers a single -32700 with a null id" do
      for {label, line} <- unparseable_shapes() do
        frame = line |> drive() |> assert_one_json_rpc_frame(label)

        assert frame["id"] == nil, "#{label}: a parse error cannot know the id"
        assert frame["error"]["code"] == -32_700, "#{label}: expected a parse error"
      end
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
