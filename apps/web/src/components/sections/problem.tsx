import { BlurFade, Card, CardContent, Section } from "@allsource/ui";
import { Clock, Database, Lock } from "lucide-react";

const problems = [
  {
    title: "State Amnesia",
    description:
      "Traditional databases only store current state. When something goes wrong, you can't see how you got there. No history means no answers, no audit trail, and no ability to replay or debug.",
    icon: Database,
  },
  {
    title: "Temporal Blindness",
    description:
      "Time-based queries are afterthoughts. Want to know what your data looked like last Tuesday at 3pm? Good luck. Most systems weren't built for time-travel, and retrofitting is painful.",
    icon: Clock,
  },
  {
    title: "AI Integration Gap",
    description:
      "Your data is trapped behind rigid APIs. AI agents can't easily explore, analyze, or manage your event streams. The tools that exist weren't designed for autonomous workflows.",
    icon: Lock,
  },
];

export default function Component() {
  return (
    <Section title="The Problem" subtitle="Event data deserves better than database tables">
      <div className="grid grid-cols-1 md:grid-cols-3 gap-8 mt-12">
        {problems.map((problem) => (
          <BlurFade key={problem.title} delay={0.2} inView>
            <Card className="bg-background border-none shadow-none">
              <CardContent className="p-6 space-y-4">
                <div className="w-12 h-12 bg-primary/10 rounded-full flex items-center justify-center">
                  <problem.icon className="w-6 h-6 text-primary" />
                </div>
                <h3 className="text-xl font-semibold">{problem.title}</h3>
                <p className="text-muted-foreground">{problem.description}</p>
              </CardContent>
            </Card>
          </BlurFade>
        ))}
      </div>
    </Section>
  );
}
