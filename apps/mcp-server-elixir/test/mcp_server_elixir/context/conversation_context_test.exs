defmodule McpServerElixir.Context.ConversationContextTest do
  use ExUnit.Case, async: false

  alias McpServerElixir.Context.ConversationContext

  @moduletag :conversation_context

  setup do
    # Start a fresh ConversationContext for each test
    name = :"context_#{:erlang.unique_integer([:positive])}"
    {:ok, pid} = ConversationContext.start_link(name: name, timeout_ms: :timer.seconds(60))
    {:ok, server: name, pid: pid}
  end

  describe "get_or_create_session/2" do
    test "creates a new session when none exists", %{server: server} do
      {:ok, session} = ConversationContext.get_or_create_session(server, "test-session-1")

      assert session.session_id == "test-session-1"
      assert session.entity_ids == []
      assert session.event_types == []
      assert session.filters == %{}
      assert %DateTime{} = session.created_at
      assert %DateTime{} = session.last_accessed
    end

    test "returns existing session when called again", %{server: server} do
      {:ok, session1} = ConversationContext.get_or_create_session(server, "test-session-2")
      {:ok, session2} = ConversationContext.get_or_create_session(server, "test-session-2")

      assert session1.session_id == session2.session_id
      assert session1.created_at == session2.created_at
    end

    test "updates last_accessed on subsequent calls", %{server: server} do
      {:ok, session1} = ConversationContext.get_or_create_session(server, "test-session-3")
      Process.sleep(10)
      {:ok, session2} = ConversationContext.get_or_create_session(server, "test-session-3")

      assert DateTime.compare(session2.last_accessed, session1.last_accessed) in [:gt, :eq]
    end
  end

  describe "get_session/2" do
    test "returns error when session doesn't exist", %{server: server} do
      assert {:error, :not_found} = ConversationContext.get_session(server, "nonexistent")
    end

    test "returns existing session", %{server: server} do
      {:ok, _} = ConversationContext.get_or_create_session(server, "test-get-session")
      {:ok, session} = ConversationContext.get_session(server, "test-get-session")

      assert session.session_id == "test-get-session"
    end
  end

  describe "update_session/3" do
    test "updates session with entity_id", %{server: server} do
      {:ok, _} = ConversationContext.get_or_create_session(server, "update-test")

      {:ok, updated} =
        ConversationContext.update_session(server, "update-test", %{entity_id: "user-123"})

      assert updated.entity_ids == ["user-123"]
    end

    test "accumulates multiple entity_ids", %{server: server} do
      {:ok, _} = ConversationContext.get_or_create_session(server, "accumulate-test")

      {:ok, _} =
        ConversationContext.update_session(server, "accumulate-test", %{entity_id: "user-1"})

      {:ok, updated} =
        ConversationContext.update_session(server, "accumulate-test", %{entity_id: "user-2"})

      assert "user-1" in updated.entity_ids
      assert "user-2" in updated.entity_ids
    end

    test "accumulates event_types", %{server: server} do
      {:ok, _} = ConversationContext.get_or_create_session(server, "event-type-test")

      {:ok, _} =
        ConversationContext.update_session(server, "event-type-test", %{
          event_type: "user.created"
        })

      {:ok, updated} =
        ConversationContext.update_session(server, "event-type-test", %{
          event_type: "user.updated"
        })

      assert "user.created" in updated.event_types
      assert "user.updated" in updated.event_types
    end

    test "merges filters", %{server: server} do
      {:ok, _} = ConversationContext.get_or_create_session(server, "filter-test")

      {:ok, _} =
        ConversationContext.update_session(server, "filter-test", %{filters: %{tier: "premium"}})

      {:ok, updated} =
        ConversationContext.update_session(server, "filter-test", %{
          filters: %{status: "active"}
        })

      assert updated.filters == %{tier: "premium", status: "active"}
    end

    test "replaces time filters", %{server: server} do
      {:ok, _} = ConversationContext.get_or_create_session(server, "time-test")

      {:ok, _} =
        ConversationContext.update_session(server, "time-test", %{since: "2024-01-01"})

      {:ok, updated} =
        ConversationContext.update_session(server, "time-test", %{since: "2024-02-01"})

      assert updated.time_since == "2024-02-01"
    end

    test "replaces entity_ids when replace_entity_ids is true", %{server: server} do
      {:ok, _} = ConversationContext.get_or_create_session(server, "replace-test")

      {:ok, _} =
        ConversationContext.update_session(server, "replace-test", %{
          entity_ids: ["user-1", "user-2"]
        })

      {:ok, updated} =
        ConversationContext.update_session(server, "replace-test", %{
          entity_ids: ["user-3"],
          replace_entity_ids: true
        })

      assert updated.entity_ids == ["user-3"]
    end

    test "handles alias since/until/as_of", %{server: server} do
      {:ok, _} = ConversationContext.get_or_create_session(server, "alias-test")

      {:ok, updated} =
        ConversationContext.update_session(server, "alias-test", %{
          since: "2024-01-01",
          until: "2024-12-31",
          as_of: "2024-06-15"
        })

      assert updated.time_since == "2024-01-01"
      assert updated.time_until == "2024-12-31"
      assert updated.time_as_of == "2024-06-15"
    end
  end

  describe "build_query/3" do
    test "builds query params from session context", %{server: server} do
      {:ok, _} = ConversationContext.get_or_create_session(server, "build-query-test")

      {:ok, _} =
        ConversationContext.update_session(server, "build-query-test", %{
          entity_id: "user-123",
          since: "2024-01-01"
        })

      {:ok, params} = ConversationContext.build_query(server, "build-query-test", %{})

      assert params["entity_id"] == "user-123"
      assert params["since"] == "2024-01-01"
    end

    test "merges refinement with existing context", %{server: server} do
      {:ok, _} = ConversationContext.get_or_create_session(server, "merge-query-test")

      {:ok, _} =
        ConversationContext.update_session(server, "merge-query-test", %{
          entity_id: "user-123"
        })

      {:ok, params} =
        ConversationContext.build_query(server, "merge-query-test", %{
          filters: %{tier: "premium"}
        })

      assert params["entity_id"] == "user-123"
      # Filters are stringified for JSON compatibility
      assert params["filters"] == %{"tier" => "premium"}
    end

    test "records query in history", %{server: server} do
      {:ok, _} = ConversationContext.get_or_create_session(server, "history-test")
      {:ok, _} = ConversationContext.build_query(server, "history-test", %{entity_id: "user-1"})

      {:ok, session} = ConversationContext.get_session(server, "history-test")

      assert length(session.query_history) == 1
      assert hd(session.query_history).refinement == %{entity_id: "user-1"}
    end

    test "limits history to 10 entries", %{server: server} do
      {:ok, _} = ConversationContext.get_or_create_session(server, "history-limit-test")

      # Add 12 queries
      for i <- 1..12 do
        {:ok, _} =
          ConversationContext.build_query(server, "history-limit-test", %{
            entity_id: "user-#{i}"
          })
      end

      {:ok, session} = ConversationContext.get_session(server, "history-limit-test")

      assert length(session.query_history) == 10
      # Most recent should be first
      assert hd(session.query_history).refinement == %{entity_id: "user-12"}
    end

    test "handles multiple entity_ids in params", %{server: server} do
      {:ok, _} = ConversationContext.get_or_create_session(server, "multi-entity-test")

      {:ok, _} =
        ConversationContext.update_session(server, "multi-entity-test", %{
          entity_ids: ["user-1", "user-2"]
        })

      {:ok, params} = ConversationContext.build_query(server, "multi-entity-test", %{})

      assert params["entity_ids"] == ["user-1", "user-2"]
      refute Map.has_key?(params, "entity_id")
    end

    test "uses entity_id for single entity", %{server: server} do
      {:ok, _} = ConversationContext.get_or_create_session(server, "single-entity-test")

      {:ok, _} =
        ConversationContext.update_session(server, "single-entity-test", %{
          entity_ids: ["user-1"]
        })

      {:ok, params} = ConversationContext.build_query(server, "single-entity-test", %{})

      assert params["entity_id"] == "user-1"
      refute Map.has_key?(params, "entity_ids")
    end
  end

  describe "record_result/3" do
    test "records result summary", %{server: server} do
      {:ok, _} = ConversationContext.get_or_create_session(server, "result-test")

      :ok =
        ConversationContext.record_result(server, "result-test", %{count: 42, event_types: ["a"]})

      {:ok, session} = ConversationContext.get_session(server, "result-test")

      assert session.last_result_summary == %{count: 42, event_types: ["a"]}
    end

    test "returns error for nonexistent session", %{server: server} do
      assert {:error, :not_found} =
               ConversationContext.record_result(server, "nonexistent", %{count: 0})
    end
  end

  describe "clear_session/2" do
    test "removes session", %{server: server} do
      {:ok, _} = ConversationContext.get_or_create_session(server, "clear-test")
      :ok = ConversationContext.clear_session(server, "clear-test")

      assert {:error, :not_found} = ConversationContext.get_session(server, "clear-test")
    end
  end

  describe "list_sessions/1" do
    test "lists all active sessions", %{server: server} do
      {:ok, _} = ConversationContext.get_or_create_session(server, "list-test-1")
      {:ok, _} = ConversationContext.get_or_create_session(server, "list-test-2")

      {:ok, _} =
        ConversationContext.update_session(server, "list-test-1", %{entity_id: "user-1"})

      sessions = ConversationContext.list_sessions(server)

      assert length(sessions) == 2
      session_ids = Enum.map(sessions, & &1.session_id)
      assert "list-test-1" in session_ids
      assert "list-test-2" in session_ids

      with_context = Enum.find(sessions, &(&1.session_id == "list-test-1"))
      assert with_context.has_context == true

      without_context = Enum.find(sessions, &(&1.session_id == "list-test-2"))
      assert without_context.has_context == false
    end
  end

  describe "get_stats/1" do
    test "returns server statistics", %{server: server} do
      {:ok, _} = ConversationContext.get_or_create_session(server, "stats-test-1")
      {:ok, _} = ConversationContext.get_or_create_session(server, "stats-test-2")

      stats = ConversationContext.get_stats(server)

      assert stats.active_sessions == 2
      assert stats.server_uptime_seconds >= 0
      assert stats.timeout_minutes == 1
    end
  end

  describe "multi-turn query workflow" do
    test "enables iterative query refinement", %{server: server} do
      session_id = "workflow-test"

      # Turn 1: Start with users created yesterday
      {:ok, _} = ConversationContext.get_or_create_session(server, session_id)

      {:ok, params1} =
        ConversationContext.build_query(server, session_id, %{
          entity_type: "user",
          since: "yesterday"
        })

      assert params1["since"] == "yesterday"

      # Turn 2: Filter to premium tier
      {:ok, params2} =
        ConversationContext.build_query(server, session_id, %{
          filters: %{tier: "premium"}
        })

      assert params2["since"] == "yesterday"
      assert params2["filters"]["tier"] == "premium"

      # Turn 3: Add specific entity for comparison
      {:ok, params3} =
        ConversationContext.build_query(server, session_id, %{
          entity_id: "user-vip-001"
        })

      assert params3["since"] == "yesterday"
      assert params3["filters"]["tier"] == "premium"
      assert params3["entity_id"] == "user-vip-001"
    end

    test "supports semantic search context", %{server: server} do
      session_id = "semantic-workflow-test"

      {:ok, _} = ConversationContext.get_or_create_session(server, session_id)

      {:ok, params1} =
        ConversationContext.build_query(server, session_id, %{
          semantic_query: "user login attempts"
        })

      assert params1["semantic_query"] == "user login attempts"

      # Refine the semantic query
      {:ok, params2} =
        ConversationContext.build_query(server, session_id, %{
          semantic_query: "failed authentication",
          since: "2024-01-01"
        })

      # Semantic query replaces
      assert params2["semantic_query"] == "failed authentication"
      assert params2["since"] == "2024-01-01"
    end
  end

  describe "session timeout" do
    test "sessions survive within timeout", %{server: server} do
      {:ok, _} = ConversationContext.get_or_create_session(server, "timeout-test")
      Process.sleep(100)

      {:ok, session} = ConversationContext.get_session(server, "timeout-test")
      assert session.session_id == "timeout-test"
    end
  end
end
