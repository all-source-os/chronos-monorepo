import { describe, expect, it, mock } from "bun:test";
import { render } from "@testing-library/react";
import { Button } from "./button";

describe("Button", () => {
  it("renders children correctly", () => {
    const { getByRole } = render(<Button>Click me</Button>);
    expect(getByRole("button")).toHaveTextContent("Click me");
  });

  it("handles click events", () => {
    const handleClick = mock();
    const { getByRole } = render(<Button onClick={handleClick}>Click me</Button>);

    // Dispatch a click event directly instead of using userEvent
    // (happy-dom has limited support for userEvent)
    getByRole("button").click();

    expect(handleClick).toHaveBeenCalledTimes(1);
  });

  it("applies variant styles correctly", () => {
    const { rerender, getByRole } = render(<Button variant="default">Default</Button>);
    expect(getByRole("button")).toHaveClass("bg-primary");

    rerender(<Button variant="destructive">Destructive</Button>);
    expect(getByRole("button")).toHaveClass("bg-destructive");

    rerender(<Button variant="outline">Outline</Button>);
    expect(getByRole("button")).toHaveClass("border");
  });

  it("applies size variants correctly", () => {
    const { rerender, getByRole } = render(<Button size="default">Default Size</Button>);
    expect(getByRole("button")).toHaveClass("h-9");

    rerender(<Button size="sm">Small</Button>);
    expect(getByRole("button")).toHaveClass("h-8");

    rerender(<Button size="lg">Large</Button>);
    expect(getByRole("button")).toHaveClass("h-10");
  });

  it("can be disabled", () => {
    const { getByRole } = render(<Button disabled>Disabled</Button>);
    expect(getByRole("button")).toBeDisabled();
  });

  it("accepts custom className", () => {
    const { getByRole } = render(<Button className="custom-class">Custom</Button>);
    expect(getByRole("button")).toHaveClass("custom-class");
  });

  it("renders as child element when asChild is true", () => {
    const { getByRole } = render(
      <Button asChild>
        <a href="/test">Link Button</a>
      </Button>
    );
    const link = getByRole("link");
    expect(link).toHaveTextContent("Link Button");
    expect(link).toHaveAttribute("href", "/test");
    expect(link).toHaveClass("bg-primary");
  });
});
