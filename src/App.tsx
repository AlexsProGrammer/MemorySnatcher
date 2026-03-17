import { useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Progress } from "@/components/ui/progress";

type DownloadProgressPayload = {
  totalFiles: number;
  completedFiles: number;
  successfulFiles: number;
  failedFiles: number;
  memoryItemId: number | null;
  status: string;
  errorMessage: string | null;
};

function App() {
  const [selectedJsonName, setSelectedJsonName] = useState<string>("");
  const [isDropzoneActive, setIsDropzoneActive] = useState(false);
  const [outputDirectory, setOutputDirectory] = useState("");
  const [isDownloading, setIsDownloading] = useState(false);
  const [needsJsonResume, setNeedsJsonResume] = useState(false);
  const [statusMessage, setStatusMessage] = useState(
    "Upload your memories JSON and choose an export folder.",
  );
  const [progressPayload, setProgressPayload] = useState<DownloadProgressPayload>({
    totalFiles: 0,
    completedFiles: 0,
    successfulFiles: 0,
    failedFiles: 0,
    memoryItemId: null,
    status: "idle",
    errorMessage: null,
  });

  const fileInputRef = useRef<HTMLInputElement | null>(null);

  useEffect(() => {
    let isMounted = true;

    const unlistenPromise = listen<DownloadProgressPayload>(
      "download-progress",
      (event) => {
        if (!isMounted) {
          return;
        }

        if (
          event.payload.status === "error" &&
          event.payload.errorMessage?.includes("403")
        ) {
          setNeedsJsonResume(true);
          setStatusMessage("Upload new JSON to resume");
        }

        setProgressPayload(event.payload);
      },
    );

    return () => {
      isMounted = false;
      void unlistenPromise.then((unlisten) => {
        unlisten();
      });
    };
  }, []);

  const progressPercent = useMemo(() => {
    if (progressPayload.totalFiles === 0) {
      return 0;
    }

    return Math.round(
      (progressPayload.completedFiles / progressPayload.totalFiles) * 100,
    );
  }, [progressPayload.completedFiles, progressPayload.totalFiles]);

  function handleFilesSelected(files: FileList | null): void {
    const file = files?.[0];
    if (!file) {
      return;
    }

    setSelectedJsonName(file.name);
    if (needsJsonResume) {
      setNeedsJsonResume(false);
      setStatusMessage(`Selected JSON: ${file.name}. Ready to resume export.`);
      return;
    }

    setStatusMessage(`Selected JSON: ${file.name}`);
  }

  async function startDownload(): Promise<void> {
    if (outputDirectory.trim().length === 0) {
      setStatusMessage("Select an export directory before starting the download.");
      return;
    }

    setIsDownloading(true);
    setStatusMessage("Downloading queued memories...");

    try {
      const downloadedCount = await invoke<number>("download_queued_memories", {
        outputDir: outputDirectory,
      });

      setStatusMessage(`Download completed. ${downloadedCount} file(s) saved.`);
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : "Unexpected download error.";

      if (errorMessage.includes("403")) {
        setNeedsJsonResume(true);
        setStatusMessage("Upload new JSON to resume");
        return;
      }

      setStatusMessage(`Download failed: ${errorMessage}`);
    } finally {
      setIsDownloading(false);
    }
  }

  const progressLabel =
    progressPayload.totalFiles > 0
      ? `${progressPayload.completedFiles}/${progressPayload.totalFiles}`
      : "0/0";

  return (
    <main className="mx-auto flex min-h-screen w-full max-w-3xl items-center px-4 py-8">
      <Card className="w-full">
        <CardHeader>
          <CardTitle>MemorySnaper Dashboard</CardTitle>
          <CardDescription>
            Upload export JSON, choose a destination folder, and track download
            progress in real time.
          </CardDescription>
        </CardHeader>

        <CardContent className="space-y-6">
          <section className="space-y-2">
            <p className="text-sm font-medium">Memories JSON</p>
            <div
              className={`rounded-lg border-2 border-dashed p-6 text-sm transition-colors ${
                isDropzoneActive ? "border-primary bg-primary/5" : "border-border"
              }`}
              onDragOver={(event) => {
                event.preventDefault();
                setIsDropzoneActive(true);
              }}
              onDragLeave={(event) => {
                event.preventDefault();
                setIsDropzoneActive(false);
              }}
              onDrop={(event) => {
                event.preventDefault();
                setIsDropzoneActive(false);
                handleFilesSelected(event.dataTransfer.files);
              }}
            >
              <p>
                {selectedJsonName.length > 0
                  ? `Selected: ${selectedJsonName}`
                  : "Drag and drop your memories_history.json here"}
              </p>

              <div className="mt-3">
                <Button
                  type="button"
                  variant="outline"
                  onClick={() => fileInputRef.current?.click()}
                >
                  Select JSON file
                </Button>
                <input
                  ref={fileInputRef}
                  className="hidden"
                  type="file"
                  accept="application/json,.json"
                  onChange={(event) => handleFilesSelected(event.target.files)}
                />
              </div>
            </div>
          </section>

          {needsJsonResume ? (
            <section className="rounded-lg border border-destructive/40 bg-destructive/10 p-3 text-sm text-destructive">
              <p className="font-medium">Export paused: Snapchat links expired.</p>
              <p className="mt-1">Upload new JSON to resume</p>
            </section>
          ) : null}

          <section className="space-y-2">
            <label className="text-sm font-medium" htmlFor="output-dir">
              Export directory
            </label>
            <input
              id="output-dir"
              value={outputDirectory}
              onChange={(event) => setOutputDirectory(event.currentTarget.value)}
              placeholder="/home/user/Pictures/MemorySnaper"
              className="h-10 w-full rounded-lg border border-input bg-background px-3 text-sm"
            />
          </section>

          <section className="space-y-2">
            <div className="flex items-center justify-between text-sm">
              <span className="font-medium">Download progress</span>
              <span>{progressLabel}</span>
            </div>
            <Progress value={progressPercent} className="h-2" />
            <p className="text-xs text-muted-foreground">
              Success: {progressPayload.successfulFiles} · Failed: {progressPayload.failedFiles}
            </p>
          </section>

          <section className="flex items-center justify-between gap-3">
            <p className="text-xs text-muted-foreground">{statusMessage}</p>
            <Button type="button" onClick={() => void startDownload()} disabled={isDownloading}>
              {isDownloading ? "Downloading..." : "Start export"}
            </Button>
          </section>
        </CardContent>
      </Card>
    </main>
  );
}

export default App;
