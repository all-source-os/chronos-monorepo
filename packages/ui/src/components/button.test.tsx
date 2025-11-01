import { describe, it, expect, mock } from "bun:test";
import { render } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Button } from "./button";

describe("Button", () => {
  it("renders children correctly", () => {
    const { getByRole } = render(<Button>Click me</Button>);
    expect(getByRole("button")).toHaveTextContent("Click me");
  });

  it("handles click events", async () => {
    const handleClick = mock();
    const user = userEvent.setup();

    const { getByRole } = render(<Button onClick={handleClick}>Click me</Button>);
    await user.click(getByRole("button"));

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
    expect(getByRole("button")).toHaveClass("h-10");

    rerender(<Button size="sm">Small</Button>);
    expect(getByRole("button")).toHaveClass("h-9");

    rerender(<Button size="lg">Large</Button>);
    expect(getByRole("button")).toHaveClass("h-11");
  });

  it("can be disabled", () => {
    const { getByRole } = render(<Button disabled>Disabled</Button>);
    expect(getByRole("button")).toBeDisabled();
  });

  it("accepts custom className", () => {
    const { getByRole } = render(<Button className="custom-class">Custom</Button>);
    expect(getByRole("button")).toHaveClass("custom-class");
  });
});
