"use client";

import { BlurFade, Card, CardContent, Section } from "@allsource/ui";
import { Clock, Database, Lock } from "lucide-react";
import { motion } from "motion/react";

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
        {problems.map((problem, index) => (
          <BlurFade key={problem.title} delay={0.2 + index * 0.1} inView>
            <motion.div whileHover={{ y: -8, transition: { duration: 0.3 } }} className="h-full">
              <Card className="h-full bg-background border border-transparent hover:border-destructive/20 shadow-none hover:shadow-lg hover:shadow-destructive/5 transition-all duration-300 group">
                <CardContent className="p-6 space-y-4">
                  <motion.div
                    className="w-12 h-12 bg-destructive/10 rounded-full flex items-center justify-center group-hover:bg-destructive/20 transition-colors duration-300"
                    whileHover={{ scale: 1.1, rotate: 5 }}
                  >
                    <problem.icon className="w-6 h-6 text-destructive transition-transform duration-300 group-hover:scale-110" />
                  </motion.div>
                  <h3 className="text-xl font-semibold group-hover:text-destructive transition-colors duration-300">
                    {problem.title}
                  </h3>
                  <p className="text-muted-foreground">{problem.description}</p>
                </CardContent>
              </Card>
            </motion.div>
          </BlurFade>
        ))}
      </div>
    </Section>
  );
}
