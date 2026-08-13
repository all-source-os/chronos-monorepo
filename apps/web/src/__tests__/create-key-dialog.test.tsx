import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { CreateKeyDialog } from "@/components/api-keys/create-key-dialog";

const SECRET = "test-api-key-value";

describe("CreateKeyDialog", () => {
  it("prefills a Chronis-scoped key and masks its returned secret", async () => {
    const onCreateKey = vi.fn().mockResolvedValue({
      id: "key-1",
      name: "Chronis sync",
      description: "Dedicated key for cn sync from this workspace.",
      key_prefix: "test",
      key: SECRET,
      scopes: ["events:read", "events:write"],
      last_used_at: null,
      expires_at: null,
      created_at: "2026-08-13T00:00:00Z",
    });

    render(
      <CreateKeyDialog
        open
        onClose={vi.fn()}
        onCreateKey={onCreateKey}
        initialName="Chronis sync"
        initialDescription="Dedicated key for cn sync from this workspace."
        initialScopes={["events:read", "events:write"]}
      />
    );

    expect(screen.getByLabelText(/Name/)).toHaveValue("Chronis sync");
    expect(screen.getByRole("button", { name: /Read Events/ })).toHaveAttribute(
      "aria-pressed",
      "true"
    );
    expect(screen.getByRole("button", { name: /Write Events/ })).toHaveAttribute(
      "aria-pressed",
      "true"
    );

    fireEvent.click(screen.getByTestId("create-key-submit"));

    await waitFor(() => expect(onCreateKey).toHaveBeenCalledTimes(1));
    const secretInput = await screen.findByLabelText("Your API Key");
    expect(secretInput).toHaveValue("•".repeat(SECRET.length));
    expect(secretInput).not.toHaveValue(SECRET);

    fireEvent.click(screen.getByRole("button", { name: "Reveal API key" }));
    expect(secretInput).toHaveValue(SECRET);
  });
});
