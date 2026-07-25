defmodule McpServerElixir.ServerErrorReportingTest do
  @moduledoc """
  Regression tests for #229's second and fourth asks: `-32603 Internal error`
  carried no information, and the stdio connection intermittently dropped.

  Both trace to the same code. The reason was attached only as JSON-RPC `data`,
  which clients routinely drop while displaying `message` — so callers saw a bare
  "Internal error". And a reason that Jason cannot encode (a struct or a tuple,
  which is what Tesla/hackney transport failures produce) raised inside the
  server GenServer, killing it and dropping the client's stdio connection instead
  of reporting the failure.
  """
  use ExUnit.Case, async: true

  alias McpServerElixir.Server

  describe "error_message/2" do
    test "names the tool and includes the underlying reason" do
      message = Server.error_message("get_stats", "HTTP 404: no route")

      assert message =~ "get_stats"
      assert message =~ "HTTP 404"
    end

    test "renders a non-string reason instead of dropping it" do
      message = Server.error_message("get_stats", {:transport, :closed})

      assert message =~ "get_stats"
      assert message =~ "closed"
    end

    test "bounds the length so a huge reason cannot flood the client" do
      message = Server.error_message("query_events", String.duplicate("x", 5_000))

      assert byte_size(message) < 1_000
    end
  end

  describe "encodable/1 keeps the server alive on any reason" do
    defmodule Weird do
      defstruct [:reason]
    end

    test "a struct reason survives JSON encoding" do
      assert {:ok, _} = Jason.encode(Server.encodable(%Weird{reason: :closed}))
    end

    test "a tuple reason survives JSON encoding" do
      # Jason raises on a bare tuple — this is the shape that crashed the server.
      assert {:error, _} = Jason.encode(%{data: {:closed, :nope}})
      assert {:ok, _} = Jason.encode(%{data: Server.encodable({:closed, :nope})})
    end

    test "passes through values that are already encodable" do
      assert Server.encodable("boom") == "boom"
      assert Server.encodable(42) == 42
      assert Server.encodable(true) == true
      assert Server.encodable(nil) == nil
      assert Server.encodable(:econnrefused) == "econnrefused"
    end

    test "recurses through maps and lists, and stringifies nested structs" do
      encoded = Server.encodable(%{"outer" => [%{"inner" => %Weird{reason: :x}}]})

      assert {:ok, json} = Jason.encode(encoded)
      assert json =~ "Weird"
    end

    test "handles a non-encodable map key" do
      assert {:ok, _} = Jason.encode(Server.encodable(%{{:tuple, :key} => "v"}))
    end
  end
end
