import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { beforeEach, describe, expect, it, vi } from "vitest";

// Mock next/navigation
const mockPush = vi.fn();
const mockSearchParams = new URLSearchParams();

vi.mock("next/navigation", () => ({
  useRouter: () => ({ push: mockPush }),
  useSearchParams: () => mockSearchParams,
}));

// Mock next/link
vi.mock("next/link", () => ({
  default: ({
    children,
    href,
    ...props
  }: {
    children: React.ReactNode;
    href: string;
    [key: string]: unknown;
  }) => (
    <a href={href} {...props}>
      {children}
    </a>
  ),
}));

// Mock @allsource/ui — passthrough divs/buttons
vi.mock("@allsource/ui", () => ({
  Button: ({
    children,
    asChild,
    ...props
  }: {
    children: React.ReactNode;
    asChild?: boolean;
    [key: string]: unknown;
  }) => {
    if (asChild) return <>{children}</>;
    return <button {...props}>{children}</button>;
  },
  Card: ({ children, ...props }: { children: React.ReactNode; [key: string]: unknown }) => (
    <div {...props}>{children}</div>
  ),
  CardContent: ({ children, ...props }: { children: React.ReactNode; [key: string]: unknown }) => (
    <div {...props}>{children}</div>
  ),
}));

vi.mock("@allsource/ui/utils", () => ({
  cn: (...args: unknown[]) => args.filter(Boolean).join(" "),
}));

// Mock lucide-react icons as simple spans
vi.mock("lucide-react", () => {
  const icon = ({ className }: { className?: string }) => <span className={className} />;
  return {
    ArrowLeft: icon,
    ArrowRight: icon,
    Check: icon,
    Code2: icon,
    Download: icon,
    Play: icon,
    Search: icon,
  };
});

import OnboardingWizardPage from "@/app/dashboard/demo/onboarding/page";

function setSearchParams(params: Record<string, string>) {
  // Clear existing
  for (const key of [...mockSearchParams.keys()]) {
    mockSearchParams.delete(key);
  }
  for (const [key, value] of Object.entries(params)) {
    mockSearchParams.set(key, value);
  }
}

beforeEach(() => {
  vi.clearAllMocks();
  setSearchParams({});
  vi.stubGlobal("fetch", vi.fn());
  vi.stubGlobal("navigator", { clipboard: { writeText: vi.fn() } });
});

// ─── SDK Snippet Correctness ─────────────────────────────────────────

describe("Install commands match actual SDK packages", () => {
  it.each([
    ["rust", "cargo add allsource"],
    ["rust", 'allsource = "0.14"'],
    ["go", "go get github.com/all-source-os/allsource-go@latest"],
    ["typescript", "bun add @allsourcedev/client"],
    ["typescript", "npm install @allsourcedev/client"],
    ["python", "pip install allsource-client"],
  ])("%s install contains: %s", (sdk, expected) => {
    setSearchParams({ sdk, step: "2" });
    render(<OnboardingWizardPage />);
    const installBlock = screen.getByTestId("install-commands");
    expect(installBlock.textContent).toContain(expected);
  });

  it("rust install does NOT reference allsource-client (wrong crate name)", () => {
    setSearchParams({ sdk: "rust", step: "2" });
    render(<OnboardingWizardPage />);
    const code = screen.getByTestId("install-commands").textContent!;
    // The Rust crate is "allsource", not "allsource-client"
    expect(code).not.toMatch(/cargo add allsource-client/);
  });
});

describe("Send event snippets use correct SDK API", () => {
  it("rust uses CoreClient::new and ingest_event(IngestEventInput{...})", () => {
    setSearchParams({ sdk: "rust", step: "3" });
    render(<OnboardingWizardPage />);
    const code = screen.getByTestId("send-event-snippet").textContent!;
    expect(code).toContain("CoreClient::new");
    expect(code).toContain("IngestEventInput");
    expect(code).toContain("ingest_event");
    expect(code).toContain("use allsource::");
  });

  it("go uses allsource.New and client.Ingest", () => {
    setSearchParams({ sdk: "go", step: "3" });
    render(<OnboardingWizardPage />);
    const code = screen.getByTestId("send-event-snippet").textContent!;
    expect(code).toContain("allsource.New");
    expect(code).toContain("client.Ingest");
    expect(code).toContain("github.com/all-source-os/allsource-go");
  });

  it("typescript uses AllSourceClient and client.ingestEvent with snake_case fields", () => {
    setSearchParams({ sdk: "typescript", step: "3" });
    render(<OnboardingWizardPage />);
    const code = screen.getByTestId("send-event-snippet").textContent!;
    expect(code).toContain('import { AllSourceClient } from "@allsourcedev/client"');
    expect(code).toContain("client.ingestEvent");
    expect(code).toContain("event_type:");
    expect(code).toContain("entity_id:");
  });

  it("python uses AllSourceClient from allsource_client and client.ingest", () => {
    setSearchParams({ sdk: "python", step: "3" });
    render(<OnboardingWizardPage />);
    const code = screen.getByTestId("send-event-snippet").textContent!;
    expect(code).toContain("from allsource_client import AllSourceClient");
    expect(code).toContain("client.ingest");
    expect(code).toContain("AllSourceClient(");
  });
});

describe("Query snippets use correct SDK API", () => {
  it("rust uses QueryClient and query_events(QueryEventsParams)", () => {
    setSearchParams({ sdk: "rust", step: "4" });
    render(<OnboardingWizardPage />);
    const code = screen.getByTestId("query-snippet").textContent!;
    expect(code).toContain("QueryClient::new");
    expect(code).toContain("QueryEventsParams::new");
    expect(code).toContain("query_events");
  });

  it("go uses client.Query with QueryOptions", () => {
    setSearchParams({ sdk: "go", step: "4" });
    render(<OnboardingWizardPage />);
    const code = screen.getByTestId("query-snippet").textContent!;
    expect(code).toContain("client.Query");
    expect(code).toContain("QueryOptions");
  });

  it("typescript uses client.queryEvents with snake_case params", () => {
    setSearchParams({ sdk: "typescript", step: "4" });
    render(<OnboardingWizardPage />);
    const code = screen.getByTestId("query-snippet").textContent!;
    expect(code).toContain("client.queryEvents");
    expect(code).toContain("event_type:");
    expect(code).toContain("entity_id:");
  });

  it("python uses client.query", () => {
    setSearchParams({ sdk: "python", step: "4" });
    render(<OnboardingWizardPage />);
    const code = screen.getByTestId("query-snippet").textContent!;
    expect(code).toContain("client.query");
    expect(code).toContain('event_type="user.signup"');
  });
});

// ─── Step Navigation ─────────────────────────────────────────────────

describe("Step navigation", () => {
  it("renders 4 step indicators", () => {
    render(<OnboardingWizardPage />);
    const indicator = screen.getByTestId("step-indicator");
    const buttons = indicator.querySelectorAll("button");
    expect(buttons).toHaveLength(4);
  });

  it("starts on step 1 with SDK selector visible", () => {
    render(<OnboardingWizardPage />);
    expect(screen.getByTestId("sdk-selector")).toBeInTheDocument();
  });

  it("shows all 4 SDK options", () => {
    render(<OnboardingWizardPage />);
    expect(screen.getByTestId("sdk-option-rust")).toBeInTheDocument();
    expect(screen.getByTestId("sdk-option-go")).toBeInTheDocument();
    expect(screen.getByTestId("sdk-option-typescript")).toBeInTheDocument();
    expect(screen.getByTestId("sdk-option-python")).toBeInTheDocument();
  });

  it("selecting an SDK navigates to step 2", () => {
    render(<OnboardingWizardPage />);
    fireEvent.click(screen.getByTestId("sdk-option-rust"));
    expect(mockPush).toHaveBeenCalledWith(expect.stringContaining("sdk=rust"));
    expect(mockPush).toHaveBeenCalledWith(expect.stringContaining("step=2"));
  });

  it("next button is disabled on step 1 without SDK selected", () => {
    render(<OnboardingWizardPage />);
    const nextBtn = screen.getByTestId("next-button");
    expect(nextBtn).toBeDisabled();
  });

  it("next button is enabled on step 1 when SDK is selected", () => {
    setSearchParams({ sdk: "rust" });
    render(<OnboardingWizardPage />);
    const nextBtn = screen.getByTestId("next-button");
    expect(nextBtn).not.toBeDisabled();
  });

  it("back button is disabled on step 1", () => {
    render(<OnboardingWizardPage />);
    expect(screen.getByTestId("back-button")).toBeDisabled();
  });

  it("shows 'choose an SDK' prompt on step 2 without SDK selected", () => {
    setSearchParams({ step: "2" });
    render(<OnboardingWizardPage />);
    expect(screen.getByText(/choose an SDK/)).toBeInTheDocument();
  });

  it("step 4 keeps dashboard completion disabled until query succeeds", () => {
    setSearchParams({ sdk: "rust", step: "4" });
    render(<OnboardingWizardPage />);
    expect(screen.getByTestId("go-to-dashboard-button")).toBeDisabled();
    expect(screen.queryByTestId("next-button")).not.toBeInTheDocument();
  });
});

// ─── Run It / Try It API Calls ───────────────────────────────────────

describe("Run It button sends correct POST to /api/v1/events", () => {
  it("posts event with correct payload", async () => {
    const mockFetch = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ event_id: "evt-123" }),
    });
    vi.stubGlobal("fetch", mockFetch);

    setSearchParams({ sdk: "typescript", step: "3" });
    render(<OnboardingWizardPage />);

    fireEvent.click(screen.getByTestId("run-it-button"));

    await waitFor(() => {
      expect(mockFetch).toHaveBeenCalledWith("/api/v1/events", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        credentials: "include",
        body: JSON.stringify({
          event_type: "user.signup",
          entity_id: "user-001",
          payload: {
            email: "dev@example.com",
            plan: "pro",
            source: "onboarding-wizard",
          },
        }),
      });
    });
  });

  it("shows success feedback with event ID", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue({
        ok: true,
        json: () => Promise.resolve({ event_id: "evt-456" }),
      })
    );

    setSearchParams({ sdk: "rust", step: "3" });
    render(<OnboardingWizardPage />);

    fireEvent.click(screen.getByTestId("run-it-button"));

    await waitFor(() => {
      expect(screen.getByTestId("event-created-feedback")).toBeInTheDocument();
      expect(screen.getByText("evt-456")).toBeInTheDocument();
    });
  });

  it("shows error on fetch failure", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue({
        ok: false,
        status: 500,
      })
    );

    setSearchParams({ sdk: "go", step: "3" });
    render(<OnboardingWizardPage />);

    fireEvent.click(screen.getByTestId("run-it-button"));

    await waitFor(() => {
      expect(screen.getByTestId("send-event-error")).toBeInTheDocument();
      expect(screen.getByText("HTTP 500")).toBeInTheDocument();
    });
  });
});

describe("Try It button sends correct GET to /api/v1/events/query", () => {
  it("queries with correct params", async () => {
    const mockFetch = vi.fn().mockResolvedValue({
      ok: true,
      json: () => Promise.resolve({ events: [{ id: "evt-1", event_type: "user.signup" }] }),
    });
    vi.stubGlobal("fetch", mockFetch);

    setSearchParams({ sdk: "python", step: "4" });
    render(<OnboardingWizardPage />);

    fireEvent.click(screen.getByTestId("try-it-button"));

    await waitFor(() => {
      const calledUrl = mockFetch.mock.calls[0]?.[0] as string;
      expect(calledUrl).toContain("/api/v1/events/query");
      expect(calledUrl).toContain("event_type=user.signup");
      expect(calledUrl).toContain("entity_id=user-001");
      expect(calledUrl).toContain("limit=10");
    });
  });

  it("shows query results", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue({
        ok: true,
        json: () => Promise.resolve({ events: [{ id: "evt-1", event_type: "user.signup" }] }),
      })
    );

    setSearchParams({ sdk: "rust", step: "4" });
    render(<OnboardingWizardPage />);

    fireEvent.click(screen.getByTestId("try-it-button"));

    await waitFor(() => {
      expect(screen.getByTestId("query-result")).toBeInTheDocument();
      expect(screen.getByText(/Found 1 event/)).toBeInTheDocument();
      expect(screen.getByTestId("go-to-dashboard-button")).not.toBeDisabled();
    });
  });

  it("does not claim verification when query returns no event", async () => {
    vi.stubGlobal(
      "fetch",
      vi.fn().mockResolvedValue({
        ok: true,
        json: () => Promise.resolve({ events: [] }),
      })
    );

    setSearchParams({ sdk: "rust", step: "4" });
    render(<OnboardingWizardPage />);

    fireEvent.click(screen.getByTestId("try-it-button"));

    await waitFor(() => {
      expect(screen.getByTestId("query-error")).toHaveTextContent(/No matching event/);
      expect(screen.getByTestId("go-to-dashboard-button")).toBeDisabled();
    });
  });

  it("shows error on query failure", async () => {
    vi.stubGlobal("fetch", vi.fn().mockResolvedValue({ ok: false, status: 403 }));

    setSearchParams({ sdk: "typescript", step: "4" });
    render(<OnboardingWizardPage />);

    fireEvent.click(screen.getByTestId("try-it-button"));

    await waitFor(() => {
      expect(screen.getByTestId("query-error")).toBeInTheDocument();
    });
  });
});
