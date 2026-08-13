import { Card, CardContent, Section } from "@allsource/ui";
import { Bot, GitBranch, History, Search } from "lucide-react";

const capabilities = [
  {
    title: "Immutable streams",
    content:
      "Keep each accepted change in sequence instead of overwriting the record that came before it.",
    icon: History,
  },
  {
    title: "Point-in-time queries",
    content:
      "Ask what a stream contained at a sequence or timestamp and reconstruct the matching state.",
    icon: Search,
  },
  {
    title: "Projection rebuilds",
    content:
      "Replay existing events into new read models without migrating or rewriting source history.",
    icon: GitBranch,
  },
  {
    title: "MCP access for agents",
    content:
      "Give an MCP client 55+ tenant-scoped tools for event reads and writes; fleet operators can enable 73 tools with administrative controls.",
    icon: Bot,
  },
];

export default function Features() {
  return (
    <Section
      title="What you can do with stored history"
      subtitle="Core capabilities"
      description="Each capability uses the same append-only event model, whether AllSource runs hosted or on your infrastructure."
    >
      <div className="grid gap-6 md:grid-cols-2">
        {capabilities.map((capability) => (
          <Card key={capability.title} className="border-border bg-card shadow-none">
            <CardContent className="flex gap-4 p-6">
              <div className="flex h-11 w-11 shrink-0 items-center justify-center border border-border bg-background">
                <capability.icon className="h-5 w-5 text-primary" aria-hidden="true" />
              </div>
              <div>
                <h3 className="text-xl font-semibold">{capability.title}</h3>
                <p className="mt-2 leading-7 text-muted-foreground">{capability.content}</p>
              </div>
            </CardContent>
          </Card>
        ))}
      </div>
    </Section>
  );
}
