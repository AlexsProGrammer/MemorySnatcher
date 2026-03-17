import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Progress } from "@/components/ui/progress";

export function DownloaderPlaceholder() {
  return (
    <Card>
      <CardHeader>
        <CardTitle>Downloader</CardTitle>
        <CardDescription>
          Placeholder workflow UI for importing and downloading memories.
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="space-y-2">
          <p className="text-sm text-muted-foreground">Download progress</p>
          <Progress value={35} className="h-2" />
        </div>

        <div className="flex gap-2">
          <Button type="button">Start Download</Button>
          <Button type="button" variant="outline">
            Process Files
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}
