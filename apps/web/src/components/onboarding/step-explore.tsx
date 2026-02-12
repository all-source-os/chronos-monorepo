"use client";

import { Button, Card, CardContent } from "@allsource/ui";
import { cn } from "@allsource/ui/utils";
import { ArrowRight, Clock, FastForward, Play, Rewind } from "lucide-react";
import { useState } from "react";

interface StepExploreProps {
  onNext: () => void;
}

interface TimelineEvent {
  id: string;
  type: string;
  time: string;
  state: Record<string, unknown>;
}

const timelineEvents: TimelineEvent[] = [
  {
    id: "1",
    type: "user.created",
    time: "10:00 AM",
    state: { name: null, email: "demo@example.com", verified: false },
  },
  {
    id: "2",
    type: "user.name_updated",
    time: "10:05 AM",
    state: { name: "John Doe", email: "demo@example.com", verified: false },
  },
  {
    id: "3",
    type: "user.verified",
    time: "10:15 AM",
    state: { name: "John Doe", email: "demo@example.com", verified: true },
  },
  {
    id: "4",
    type: "user.upgraded",
    time: "11:00 AM",
    state: { name: "John Doe", email: "demo@example.com", verified: true, plan: "pro" },
  },
];

export function StepExplore({ onNext }: StepExploreProps) {
  const [currentIndex, setCurrentIndex] = useState(timelineEvents.length - 1);
  const [isPlaying, setIsPlaying] = useState(false);

  const currentEvent = timelineEvents[currentIndex]!;
  const progress = ((currentIndex + 1) / timelineEvents.length) * 100;

  const goBack = () => {
    if (currentIndex > 0) {
      setCurrentIndex(currentIndex - 1);
    }
  };

  const goForward = () => {
    if (currentIndex < timelineEvents.length - 1) {
      setCurrentIndex(currentIndex + 1);
    }
  };

  const handlePlay = () => {
    if (isPlaying) {
      setIsPlaying(false);
      return;
    }

    setIsPlaying(true);
    setCurrentIndex(0);

    let index = 0;
    const interval = setInterval(() => {
      index++;
      if (index >= timelineEvents.length) {
        clearInterval(interval);
        setIsPlaying(false);
      } else {
        setCurrentIndex(index);
      }
    }, 1000);
  };

  return (
    <div className="mx-auto max-w-2xl">
      <div className="mb-8 text-center">
        <h2 className="mb-2 text-2xl font-bold">Time Travel Through Events</h2>
        <p className="text-muted-foreground">
          See how your entity's state evolves over time. Replay history to debug issues.
        </p>
      </div>

      {/* Timeline visualization */}
      <Card className="mb-6">
        <CardContent className="p-6">
          {/* Progress bar */}
          <div className="mb-6">
            <div className="mb-2 flex justify-between text-xs text-muted-foreground">
              <span>Beginning</span>
              <span>Now</span>
            </div>
            <div className="relative h-2 rounded-full bg-muted">
              <div
                className="absolute left-0 top-0 h-full rounded-full bg-primary transition-all duration-300"
                style={{ width: `${progress}%` }}
              />
              {timelineEvents.map((event, index) => (
                <button
                  key={event.id}
                  onClick={() => setCurrentIndex(index)}
                  className={cn(
                    "absolute top-1/2 h-4 w-4 -translate-x-1/2 -translate-y-1/2 rounded-full border-2 border-background transition-all",
                    index <= currentIndex ? "bg-primary" : "bg-muted"
                  )}
                  style={{
                    left: `${((index + 1) / timelineEvents.length) * 100}%`,
                  }}
                />
              ))}
            </div>
          </div>

          {/* Controls */}
          <div className="mb-6 flex items-center justify-center gap-4">
            <Button
              variant="outline"
              size="icon"
              onClick={goBack}
              disabled={currentIndex === 0 || isPlaying}
            >
              <Rewind className="h-4 w-4" />
            </Button>
            <Button
              variant="outline"
              size="icon"
              onClick={handlePlay}
              className={cn(isPlaying && "bg-primary text-primary-foreground")}
            >
              <Play className="h-4 w-4" />
            </Button>
            <Button
              variant="outline"
              size="icon"
              onClick={goForward}
              disabled={currentIndex === timelineEvents.length - 1 || isPlaying}
            >
              <FastForward className="h-4 w-4" />
            </Button>
          </div>

          {/* Current event display */}
          <div className="rounded-lg bg-muted/50 p-4">
            <div className="mb-3 flex items-center justify-between">
              <span className="rounded-full bg-primary/10 px-3 py-1 text-sm font-medium text-primary">
                {currentEvent.type}
              </span>
              <span className="flex items-center gap-1.5 text-sm text-muted-foreground">
                <Clock className="h-3.5 w-3.5" />
                {currentEvent.time}
              </span>
            </div>
            <div>
              <p className="mb-1 text-xs font-medium text-muted-foreground">
                Entity State at this point:
              </p>
              <pre className="rounded-lg bg-background p-3 text-xs">
                <code>{JSON.stringify(currentEvent.state, null, 2)}</code>
              </pre>
            </div>
          </div>
        </CardContent>
      </Card>

      {/* Info box */}
      <div className="mb-6 rounded-lg border border-primary/20 bg-primary/5 p-4">
        <h3 className="mb-1 font-medium">Why Time Travel Matters</h3>
        <p className="text-sm text-muted-foreground">
          Traditional databases only show current state. With AllSource, you can see exactly what
          happened, when it happened, and reconstruct state at any point in time. Perfect for
          debugging, auditing, and understanding user journeys.
        </p>
      </div>

      {/* Continue button */}
      <div className="flex justify-center">
        <Button size="lg" onClick={onNext}>
          Continue
          <ArrowRight className="ml-2 h-4 w-4" />
        </Button>
      </div>
    </div>
  );
}
