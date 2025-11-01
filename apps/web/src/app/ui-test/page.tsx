import { Button, Card, CardHeader, CardTitle, CardContent, Badge } from "@allsource/ui";

export default function UITestPage() {
  return (
    <div className="container mx-auto p-8">
      <h1 className="text-3xl font-bold mb-8">UI Component Test</h1>

      <div className="space-y-6">
        <div>
          <h2 className="text-xl font-semibold mb-4">Buttons</h2>
          <div className="flex gap-4">
            <Button variant="default">Default Button</Button>
            <Button variant="destructive">Destructive</Button>
            <Button variant="outline">Outline</Button>
            <Button variant="secondary">Secondary</Button>
            <Button variant="ghost">Ghost</Button>
          </div>
        </div>

        <div>
          <h2 className="text-xl font-semibold mb-4">Badges</h2>
          <div className="flex gap-4">
            <Badge>Default</Badge>
            <Badge variant="secondary">Secondary</Badge>
            <Badge variant="destructive">Destructive</Badge>
            <Badge variant="outline">Outline</Badge>
          </div>
        </div>

        <div>
          <h2 className="text-xl font-semibold mb-4">Cards</h2>
          <Card className="max-w-md">
            <CardHeader>
              <CardTitle>Test Card</CardTitle>
            </CardHeader>
            <CardContent>This card is imported from @allsource/ui package!</CardContent>
          </Card>
        </div>
      </div>
    </div>
  );
}
