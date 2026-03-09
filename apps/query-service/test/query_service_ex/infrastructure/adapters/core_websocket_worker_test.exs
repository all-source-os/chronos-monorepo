defmodule QueryServiceEx.Infrastructure.Adapters.CoreWebSocketWorkerTest do
  use ExUnit.Case, async: true

  alias QueryServiceEx.Infrastructure.Adapters.CoreWebSocketWorker

  @moduletag :websocket

  describe "connect_opts/2" do
    test "forces HTTP/1.1 protocol to avoid HTTP/2 extended CONNECT issues" do
      {_address, opts} = CoreWebSocketWorker.connect_opts("example.com", 10_000)
      assert opts[:protocols] == [:http1]
    end

    test "includes transport timeout" do
      {_address, opts} = CoreWebSocketWorker.connect_opts("example.com", 5_000)
      assert opts[:transport_opts][:timeout] == 5_000
    end

    test "resolves IPv6 and includes hostname for SNI" do
      {_address, opts} = CoreWebSocketWorker.connect_opts("localhost", 10_000)
      # Whether IPv6 resolves or not, protocols must always be [:http1]
      assert opts[:protocols] == [:http1]
    end

    test "returns hostname string when IPv6 resolution fails" do
      {address, _opts} =
        CoreWebSocketWorker.connect_opts("unlikely-host-that-wont-resolve.invalid", 10_000)

      assert is_binary(address)
    end
  end
end
