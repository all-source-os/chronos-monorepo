import { render, screen } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import MarketingLayout from "@/app/(marketing)/layout";
import { VsPageBody } from "@/app/(marketing)/vs/_components/VsPageBody";
import { competitors } from "@/app/(marketing)/vs/_data/competitors";

describe("comparison page layout", () => {
  it("uses the marketing shell exactly once", () => {
    render(
      <MarketingLayout>
        <VsPageBody competitor={competitors.mem0} />
      </MarketingLayout>
    );

    expect(screen.getAllByRole("banner")).toHaveLength(1);
    expect(screen.getAllByRole("main")).toHaveLength(1);
    expect(screen.getAllByRole("contentinfo")).toHaveLength(1);
  });
});
