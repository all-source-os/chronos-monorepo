import { describe, expect, it } from "bun:test";
import { render } from "@testing-library/react";
import { Badge } from "./badge";

describe("Badge", () => {
  it("renders children correctly", () => {
    const { getByText } = render(<Badge>Badge text</Badge>);
    expect(getByText("Badge text")).toBeInTheDocument();
  });

  it("renders as span element for inline semantics", () => {
    const { container } = render(<Badge>Badge</Badge>);
    const span = container.querySelector("span");
    expect(span).toBeTruthy();
    expect(span?.textContent).toBe("Badge");
  });

  it("applies default variant classes", () => {
    const { getByTestId } = render(<Badge data-testid="badge">Default</Badge>);
    const badge = getByTestId("badge");
    expect(badge).toHaveClass("bg-primary", "text-primary-foreground");
  });

  it("applies secondary variant classes", () => {
    const { getByTestId } = render(
      <Badge variant="secondary" data-testid="badge">
        Secondary
      </Badge>
    );
    const badge = getByTestId("badge");
    expect(badge).toHaveClass("bg-secondary", "text-secondary-foreground");
  });

  it("applies destructive variant classes", () => {
    const { getByTestId } = render(
      <Badge variant="destructive" data-testid="badge">
        Destructive
      </Badge>
    );
    const badge = getByTestId("badge");
    expect(badge).toHaveClass("bg-destructive", "text-destructive-foreground");
  });

  it("applies outline variant classes", () => {
    const { getByTestId } = render(
      <Badge variant="outline" data-testid="badge">
        Outline
      </Badge>
    );
    const badge = getByTestId("badge");
    expect(badge).toHaveClass("border", "border-input");
  });

  it("accepts custom className", () => {
    const { getByTestId } = render(
      <Badge className="custom-badge" data-testid="badge">
        Custom
      </Badge>
    );
    const badge = getByTestId("badge");
    expect(badge).toHaveClass("custom-badge");
  });

  it("applies base classes to all variants", () => {
    const { getByTestId } = render(<Badge data-testid="badge">Badge</Badge>);
    const badge = getByTestId("badge");
    expect(badge).toHaveClass(
      "inline-flex",
      "items-center",
      "rounded-md",
      "px-2",
      "py-1",
      "text-xs",
      "font-semibold"
    );
  });
});
