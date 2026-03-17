import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";

export function ViewerPlaceholder() {
  return (
    <Card>
      <CardHeader>
        <CardTitle>Viewer</CardTitle>
        <CardDescription>
          Placeholder for the virtualized media grid and thumbnail previews.
        </CardDescription>
      </CardHeader>
      <CardContent className="flex items-center justify-between gap-3">
        <p className="text-sm text-muted-foreground">No thumbnails loaded yet.</p>
        <Button type="button" variant="outline">
          Refresh
        </Button>
      </CardContent>
    </Card>
  );
}
