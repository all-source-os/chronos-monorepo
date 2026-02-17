defmodule QueryServiceEx.Projections.ProjectionSupervisorTest do
  use ExUnit.Case, async: false

  alias QueryServiceEx.Projections.ProjectionSupervisor

  setup do
    # Stop the app-level "indices" projection to free up the via name
    # so tests can start their own instances
    ProjectionSupervisor.stop_projection("indices")

    # Clean up ETS tables from previous tests
    for table <- [:projection_index, :projection_test_sup_a] do
      if :ets.whereis(table) != :undefined do
        :ets.delete(table)
      end
    end

    on_exit(fn ->
      # Clean up ETS tables
      for table <- [:projection_index, :projection_test_sup_a] do
        if :ets.whereis(table) != :undefined do
          :ets.delete(table)
        end
      end

      # Restart the app-level projection
      ProjectionSupervisor.start_projection("indices")
    end)

    :ok
  end

  describe "start_link/1" do
    test "starts the DynamicSupervisor" do
      {:ok, pid} = ProjectionSupervisor.start_link(name: :test_sup_start)
      assert Process.alive?(pid)
      DynamicSupervisor.stop(pid)
    end
  end

  describe "start_projection/2 and stop_projection/2" do
    test "starts a projection server for a registered projection" do
      {:ok, sup} = DynamicSupervisor.start_link(strategy: :one_for_one)

      {:ok, child_pid} =
        ProjectionSupervisor.start_projection("indices", sup)

      assert Process.alive?(child_pid)

      DynamicSupervisor.stop(sup)
    end

    test "returns error for unknown projection" do
      {:ok, sup} = DynamicSupervisor.start_link(strategy: :one_for_one)

      assert {:error, :unknown_projection} =
               ProjectionSupervisor.start_projection("nonexistent", sup)

      DynamicSupervisor.stop(sup)
    end

    test "stop_projection terminates a running projection server" do
      {:ok, sup} = DynamicSupervisor.start_link(strategy: :one_for_one)

      {:ok, child_pid} =
        ProjectionSupervisor.start_projection("indices", sup)

      assert Process.alive?(child_pid)
      assert :ok = ProjectionSupervisor.stop_projection("indices", sup)

      Process.sleep(10)
      refute Process.alive?(child_pid)

      DynamicSupervisor.stop(sup)
    end

    test "stop_projection returns error for non-running projection" do
      {:ok, sup} = DynamicSupervisor.start_link(strategy: :one_for_one)

      assert {:error, :not_found} =
               ProjectionSupervisor.stop_projection("indices", sup)

      DynamicSupervisor.stop(sup)
    end
  end

  describe "start_all/1" do
    test "starts children for all registered projections" do
      {:ok, sup} = DynamicSupervisor.start_link(strategy: :one_for_one)
      ProjectionSupervisor.start_all(sup)

      Process.sleep(50)

      children = DynamicSupervisor.which_children(sup)
      # At least one child (indices is registered in Registry)
      assert length(children) >= 1

      DynamicSupervisor.stop(sup)
    end
  end

  describe "supervisor restarts crashed children" do
    test "crashed ProjectionServer is restarted by supervisor" do
      {:ok, sup} = ProjectionSupervisor.start_link(name: :test_sup_restart)
      Process.sleep(100)

      children = DynamicSupervisor.which_children(sup)

      if length(children) > 0 do
        {_, child_pid, _, _} = hd(children)

        # Kill the child
        Process.exit(child_pid, :kill)
        Process.sleep(200)

        # Supervisor should have restarted it
        new_children = DynamicSupervisor.which_children(sup)
        assert length(new_children) >= 1

        {_, new_pid, _, _} = hd(new_children)
        assert new_pid != child_pid
        assert Process.alive?(new_pid)
      end

      DynamicSupervisor.stop(sup)
    end
  end
end
