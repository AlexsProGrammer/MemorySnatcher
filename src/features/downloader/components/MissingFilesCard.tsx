import { Download, FileWarning } from "lucide-react";

import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import type { MissingFileItem } from "@/lib/memories-api";

type MissingFilesCardProps = {
  items: MissingFileItem[];
  isLoading: boolean;
  isDownloading: boolean;
  canDownloadAll: boolean;
  onDownloadAll: () => void;
};

function MissingFileRow({ item }: { item: MissingFileItem }) {
  return (
    <div className="rounded-md border bg-background/60 p-2">
      <div className="flex items-center justify-between gap-2">
        <span className="text-xs font-medium truncate">{item.dateTaken}</span>
        {item.mid ? (
          <Badge variant="outline" className="text-[10px]">{item.mid.slice(0, 8)}</Badge>
        ) : null}
      </div>
      {item.location ? (
        <p className="text-[11px] text-muted-foreground truncate">{item.location}</p>
      ) : null}
      <p className="text-[11px] text-muted-foreground truncate" title={item.mediaDownloadUrl}>
        {item.mediaDownloadUrl}
      </p>
      {item.lastErrorMessage ? (
        <p className="text-[11px] text-amber-600 dark:text-amber-400 truncate" title={item.lastErrorMessage}>
          {item.lastErrorMessage}
        </p>
      ) : null}
    </div>
  );
}

export function MissingFilesCard({
  items,
  isLoading,
  isDownloading,
  canDownloadAll,
  onDownloadAll,
}: MissingFilesCardProps) {
  return (
    <Card>
      <CardHeader className="pb-2">
        <CardTitle className="flex items-center gap-2 text-sm">
          <FileWarning className="h-4 w-4 text-amber-600 dark:text-amber-400" />
          Missing files ({items.length})
        </CardTitle>
      </CardHeader>
      <CardContent className="space-y-3">
        <div className="max-h-72 space-y-2 overflow-y-auto pr-1">
          {isLoading ? (
            <p className="text-xs text-muted-foreground">Loading missing file list...</p>
          ) : items.length === 0 ? (
            <p className="text-xs text-muted-foreground">No missing files found.</p>
          ) : (
            items.map((item) => (
              <MissingFileRow key={item.memoryGroupId} item={item} />
            ))
          )}
        </div>

        <Button
          type="button"
          onClick={onDownloadAll}
          disabled={isDownloading || isLoading || items.length === 0 || !canDownloadAll}
          className="w-full gap-2"
        >
          <Download className="h-4 w-4" />
          {isDownloading ? "Downloading..." : "Download All"}
        </Button>
        {!canDownloadAll ? (
          <p className="text-[11px] text-muted-foreground">
            Available after ZIP processing is stopped or finished.
          </p>
        ) : null}
      </CardContent>
    </Card>
  );
}
