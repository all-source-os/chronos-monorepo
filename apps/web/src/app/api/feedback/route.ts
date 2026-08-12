import { type NextRequest, NextResponse } from "next/server";
import { logSafe } from "@/lib/log-safe";

const GITHUB_OWNER = "all-source-os";
const GITHUB_REPO = "all-source";

const categoryLabels: Record<string, string> = {
  bug: "bug",
  feature: "enhancement",
  question: "question",
};

export async function POST(request: NextRequest) {
  let body: { category?: string; message?: string; email?: string };
  try {
    body = await request.json();
  } catch {
    return NextResponse.json({ error: "Invalid JSON body" }, { status: 400 });
  }

  const { category, message, email } = body;

  if (!category || !["bug", "feature", "question"].includes(category)) {
    return NextResponse.json({ error: "Invalid category" }, { status: 400 });
  }

  if (!message || typeof message !== "string" || message.trim().length === 0) {
    return NextResponse.json({ error: "Message is required" }, { status: 400 });
  }

  if (message.length > 5000) {
    return NextResponse.json({ error: "Message too long (max 5000 characters)" }, { status: 400 });
  }

  const githubToken = process.env.GITHUB_FEEDBACK_TOKEN;

  // Build the issue body
  const issueBody = [
    `**Category:** ${category}`,
    email ? `**Contact:** ${email}` : null,
    "",
    "---",
    "",
    message.trim(),
    "",
    "---",
    `*Submitted via in-app feedback widget at ${new Date().toISOString()}*`,
  ]
    .filter((line) => line !== null)
    .join("\n");

  const titlePrefix =
    category === "bug" ? "[Bug]" : category === "feature" ? "[Feature]" : "[Question]";
  const titlePreview = message.trim().slice(0, 80).replace(/\n/g, " ");
  const issueTitle = `${titlePrefix} ${titlePreview}${message.trim().length > 80 ? "..." : ""}`;

  // If GitHub token is available, create a GitHub issue
  if (githubToken) {
    try {
      const res = await fetch(
        `https://api.github.com/repos/${GITHUB_OWNER}/${GITHUB_REPO}/issues`,
        {
          method: "POST",
          headers: {
            Authorization: `token ${githubToken}`,
            Accept: "application/vnd.github.v3+json",
            "Content-Type": "application/json",
          },
          body: JSON.stringify({
            title: issueTitle,
            body: issueBody,
            labels: ["feedback", categoryLabels[category]].filter(Boolean),
          }),
        }
      );

      if (!res.ok) {
        console.error("GitHub API error:", res.status, await res.text());
        return NextResponse.json({ error: "Failed to submit feedback" }, { status: 502 });
      }

      return NextResponse.json({ ok: true });
    } catch (err) {
      console.error("GitHub API request failed:", err);
      return NextResponse.json({ error: "Failed to submit feedback" }, { status: 502 });
    }
  }

  // Fallback: log to console (useful during development)
  console.log("=== Feedback Received ===");
  console.log("Title:", logSafe(issueTitle));
  console.log("Body:", logSafe(issueBody));
  console.log("========================");

  return NextResponse.json({ ok: true });
}
