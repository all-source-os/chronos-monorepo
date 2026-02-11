"use client";

import { useState } from "react";
import { Button, Card, CardContent, Input, Label } from "@allsource/ui";
import { cn } from "@allsource/ui/utils";
import { Loader2, Check, Sparkles, Code2 } from "lucide-react";
import { useOnboarding } from "@/hooks/use-onboarding";

interface StepCreateEventProps {
  onNext: () => void;
}

const eventTemplates = [
  {
    name: "User Signed Up",
    entity_id: "user-demo-001",
    event_type: "user.signed_up",
    payload: { email: "demo@example.com", plan: "free" },
  },
  {
    name: "Order Placed",
    entity_id: "order-demo-001",
    event_type: "order.placed",
    payload: { items: 3, total: 99.99 },
  },
  {
    name: "Payment Completed",
    entity_id: "payment-demo-001",
    event_type: "payment.completed",
    payload: { method: "card", amount: 99.99 },
  },
];

export function StepCreateEvent({ onNext }: StepCreateEventProps) {
  const { setEventCreated } = useOnboarding();
  const [selectedTemplate, setSelectedTemplate] = useState(0);
  const [isCreating, setIsCreating] = useState(false);
  const [isCreated, setIsCreated] = useState(false);
  const [showCode, setShowCode] = useState(false);

  const currentTemplate = eventTemplates[selectedTemplate]!;

  const handleCreate = async () => {
    setIsCreating(true);
    // Simulate API call
    await new Promise((resolve) => setTimeout(resolve, 1500));
    setIsCreating(false);
    setIsCreated(true);
    setEventCreated(true);

    // Auto-advance after showing success
    setTimeout(() => {
      onNext();
    }, 2000);
  };

  const codeSnippet = `// Create an event using the REST API
const response = await fetch('https://api.allsource.dev/api/events', {
  method: 'POST',
  headers: {
    'Content-Type': 'application/json',
    'Authorization': 'Bearer YOUR_API_KEY'
  },
  body: JSON.stringify({
    entity_id: '${currentTemplate.entity_id}',
    event_type: '${currentTemplate.event_type}',
    payload: ${JSON.stringify(currentTemplate.payload, null, 4).replace(/\n/g, '\n    ')}
  })
});`;

  return (
    <div className="mx-auto max-w-2xl">
      <div className="mb-8 text-center">
        <h2 className="mb-2 text-2xl font-bold">Create Your First Event</h2>
        <p className="text-muted-foreground">
          Events are the building blocks of your data. Let's create one now.
        </p>
      </div>

      {/* Template selection */}
      <div className="mb-6">
        <Label className="mb-3 block">Choose an event template</Label>
        <div className="grid gap-3 sm:grid-cols-3">
          {eventTemplates.map((template, index) => (
            <button
              key={template.name}
              onClick={() => setSelectedTemplate(index)}
              className={cn(
                "rounded-lg border p-4 text-left transition-all",
                selectedTemplate === index
                  ? "border-primary bg-primary/5"
                  : "border-border hover:border-primary/50"
              )}
              disabled={isCreating || isCreated}
            >
              <p className="font-medium">{template.name}</p>
              <p className="text-xs text-muted-foreground">
                {template.event_type}
              </p>
            </button>
          ))}
        </div>
      </div>

      {/* Event preview */}
      <Card className="mb-6">
        <CardContent className="p-4">
          <div className="mb-4 flex items-center justify-between">
            <span className="text-sm font-medium">Event Preview</span>
            <Button
              variant="ghost"
              size="sm"
              onClick={() => setShowCode(!showCode)}
            >
              <Code2 className="mr-1.5 h-4 w-4" />
              {showCode ? "Hide code" : "Show code"}
            </Button>
          </div>

          {showCode ? (
            <pre className="overflow-x-auto rounded-lg bg-muted p-4 text-xs">
              <code>{codeSnippet}</code>
            </pre>
          ) : (
            <div className="space-y-3">
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <Label className="text-xs text-muted-foreground">
                    Entity ID
                  </Label>
                  <Input value={currentTemplate.entity_id} readOnly />
                </div>
                <div>
                  <Label className="text-xs text-muted-foreground">
                    Event Type
                  </Label>
                  <Input value={currentTemplate.event_type} readOnly />
                </div>
              </div>
              <div>
                <Label className="text-xs text-muted-foreground">Payload</Label>
                <pre className="mt-1 rounded-lg bg-muted p-3 text-xs">
                  {JSON.stringify(currentTemplate.payload, null, 2)}
                </pre>
              </div>
            </div>
          )}
        </CardContent>
      </Card>

      {/* Create button */}
      <div className="flex justify-center">
        <Button
          size="lg"
          onClick={handleCreate}
          disabled={isCreating || isCreated}
          className="min-w-[200px]"
        >
          {isCreated ? (
            <>
              <Check className="mr-2 h-4 w-4" />
              Event Created!
            </>
          ) : isCreating ? (
            <>
              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              Creating...
            </>
          ) : (
            <>
              <Sparkles className="mr-2 h-4 w-4" />
              Create Event
            </>
          )}
        </Button>
      </div>

      {isCreated && (
        <div className="mt-6 text-center">
          <p className="text-sm text-green-600 dark:text-green-400">
            Your first event has been recorded. Moving to the next step...
          </p>
        </div>
      )}
    </div>
  );
}
