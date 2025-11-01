import { describe, it, expect } from "bun:test";
import { render } from "@testing-library/react";
import { Card, CardHeader, CardTitle, CardDescription, CardContent, CardFooter } from "./card";

describe("Card", () => {
  it("renders children correctly", () => {
    const { getByTestId } = render(<Card data-testid="card">Card content</Card>);
    const card = getByTestId("card");
    expect(card).toHaveTextContent("Card content");
  });

  it("applies correct base classes", () => {
    const { getByTestId } = render(<Card data-testid="card">Content</Card>);
    const card = getByTestId("card");
    expect(card).toHaveClass(
      "bg-card",
      "text-card-foreground",
      "rounded-xl",
      "border",
      "shadow-sm"
    );
  });

  it("accepts custom className", () => {
    const { getByTestId } = render(
      <Card className="custom-class" data-testid="card">
        Content
      </Card>
    );
    const card = getByTestId("card");
    expect(card).toHaveClass("custom-class");
  });

  it("has correct data-slot attribute", () => {
    const { getByTestId } = render(<Card data-testid="card">Content</Card>);
    const card = getByTestId("card");
    expect(card).toHaveAttribute("data-slot", "card");
  });
});

describe("CardHeader", () => {
  it("renders children correctly", () => {
    const { getByTestId } = render(<CardHeader data-testid="header">Header content</CardHeader>);
    const header = getByTestId("header");
    expect(header).toHaveTextContent("Header content");
  });

  it("applies correct base classes", () => {
    const { getByTestId } = render(<CardHeader data-testid="header">Content</CardHeader>);
    const header = getByTestId("header");
    expect(header).toHaveClass("px-6");
  });

  it("accepts custom className", () => {
    const { getByTestId } = render(
      <CardHeader className="custom-header" data-testid="header">
        Content
      </CardHeader>
    );
    const header = getByTestId("header");
    expect(header).toHaveClass("custom-header");
  });

  it("has correct data-slot attribute", () => {
    const { getByTestId } = render(<CardHeader data-testid="header">Content</CardHeader>);
    const header = getByTestId("header");
    expect(header).toHaveAttribute("data-slot", "card-header");
  });
});

describe("CardTitle", () => {
  it("renders children correctly", () => {
    const { getByTestId } = render(<CardTitle data-testid="title">Card Title</CardTitle>);
    const title = getByTestId("title");
    expect(title).toHaveTextContent("Card Title");
  });

  it("applies correct base classes", () => {
    const { getByTestId } = render(<CardTitle data-testid="title">Title</CardTitle>);
    const title = getByTestId("title");
    expect(title).toHaveClass("font-semibold", "leading-none");
  });

  it("accepts custom className", () => {
    const { getByTestId } = render(
      <CardTitle className="text-lg" data-testid="title">
        Title
      </CardTitle>
    );
    const title = getByTestId("title");
    expect(title).toHaveClass("text-lg");
  });

  it("has correct data-slot attribute", () => {
    const { getByTestId } = render(<CardTitle data-testid="title">Title</CardTitle>);
    const title = getByTestId("title");
    expect(title).toHaveAttribute("data-slot", "card-title");
  });
});

describe("CardDescription", () => {
  it("renders children correctly", () => {
    const { getByTestId } = render(
      <CardDescription data-testid="desc">Description text</CardDescription>
    );
    const desc = getByTestId("desc");
    expect(desc).toHaveTextContent("Description text");
  });

  it("applies correct base classes", () => {
    const { getByTestId } = render(
      <CardDescription data-testid="desc">Description</CardDescription>
    );
    const desc = getByTestId("desc");
    expect(desc).toHaveClass("text-muted-foreground", "text-sm");
  });

  it("accepts custom className", () => {
    const { getByTestId } = render(
      <CardDescription className="text-xs" data-testid="desc">
        Description
      </CardDescription>
    );
    const desc = getByTestId("desc");
    expect(desc).toHaveClass("text-xs");
  });

  it("has correct data-slot attribute", () => {
    const { getByTestId } = render(
      <CardDescription data-testid="desc">Description</CardDescription>
    );
    const desc = getByTestId("desc");
    expect(desc).toHaveAttribute("data-slot", "card-description");
  });
});

describe("CardContent", () => {
  it("renders children correctly", () => {
    const { getByTestId } = render(<CardContent data-testid="content">Content text</CardContent>);
    const content = getByTestId("content");
    expect(content).toHaveTextContent("Content text");
  });

  it("applies correct base classes", () => {
    const { getByTestId } = render(<CardContent data-testid="content">Content</CardContent>);
    const content = getByTestId("content");
    expect(content).toHaveClass("px-6");
  });

  it("accepts custom className", () => {
    const { getByTestId } = render(
      <CardContent className="py-4" data-testid="content">
        Content
      </CardContent>
    );
    const content = getByTestId("content");
    expect(content).toHaveClass("py-4");
  });

  it("has correct data-slot attribute", () => {
    const { getByTestId } = render(<CardContent data-testid="content">Content</CardContent>);
    const content = getByTestId("content");
    expect(content).toHaveAttribute("data-slot", "card-content");
  });
});

describe("CardFooter", () => {
  it("renders children correctly", () => {
    const { getByTestId } = render(<CardFooter data-testid="footer">Footer content</CardFooter>);
    const footer = getByTestId("footer");
    expect(footer).toHaveTextContent("Footer content");
  });

  it("applies correct base classes", () => {
    const { getByTestId } = render(<CardFooter data-testid="footer">Footer</CardFooter>);
    const footer = getByTestId("footer");
    expect(footer).toHaveClass("flex", "items-center", "px-6");
  });

  it("accepts custom className", () => {
    const { getByTestId } = render(
      <CardFooter className="justify-end" data-testid="footer">
        Footer
      </CardFooter>
    );
    const footer = getByTestId("footer");
    expect(footer).toHaveClass("justify-end");
  });

  it("has correct data-slot attribute", () => {
    const { getByTestId } = render(<CardFooter data-testid="footer">Footer</CardFooter>);
    const footer = getByTestId("footer");
    expect(footer).toHaveAttribute("data-slot", "card-footer");
  });
});

describe("Card composition", () => {
  it("renders a complete card with all sub-components", () => {
    const { getByTestId } = render(
      <Card data-testid="card">
        <CardHeader data-testid="header">
          <CardTitle data-testid="title">Test Card</CardTitle>
          <CardDescription data-testid="description">Test description</CardDescription>
        </CardHeader>
        <CardContent data-testid="content">Test content</CardContent>
        <CardFooter data-testid="footer">Test footer</CardFooter>
      </Card>
    );

    expect(getByTestId("card")).toBeInTheDocument();
    expect(getByTestId("header")).toBeInTheDocument();
    expect(getByTestId("title")).toHaveTextContent("Test Card");
    expect(getByTestId("description")).toHaveTextContent("Test description");
    expect(getByTestId("content")).toHaveTextContent("Test content");
    expect(getByTestId("footer")).toHaveTextContent("Test footer");
  });
});
