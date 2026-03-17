import { useEffect, useMemo, useRef, useState } from "react";

import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Progress } from "@/components/ui/progress";
import {
  type ExportJobState,
  getJobState,
  processDownloadedMemories,
  downloadQueuedMemories,
  importMemoriesJson,
  onDownloadProgress,
  resumeExportDownloads,
  type DownloadProgressPayload,
} from "@/lib/memories-api";

type WorkflowState =
  | "IDLE"
  | "IMPORTED"
  | "RUNNING"
  | "PAUSED_EXPIRED"
  | "PAUSED_ERROR"
  | "RESUMABLE"
  | "COMPLETED";

function App() {
  const [selectedJsonName, setSelectedJsonName] = useState<string>("");
  const [isDropzoneActive, setIsDropzoneActive] = useState(false);
  const [outputDirectory, setOutputDirectory] = useState("");
  const [isDownloading, setIsDownloading] = useState(false);
  const [isProcessing, setIsProcessing] = useState(false);
  const [keepOriginals, setKeepOriginals] = useState(false);
  const [needsJsonResume, setNeedsJsonResume] = useState(false);
  const [workflowState, setWorkflowState] = useState<WorkflowState>("IDLE");
  const [jobState, setJobState] = useState<ExportJobState | null>(null);
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
    errorCode: null,
    errorMessage: null,
  });

  const fileInputRef = useRef<HTMLInputElement | null>(null);

  useEffect(() => {
    let isMounted = true;

    const applyJobState = (nextJobState: ExportJobState): void => {
      setJobState(nextJobState);

      switch (nextJobState.status) {
        case "running":
          setWorkflowState("RUNNING");
          break;
        case "paused_expired":
          setNeedsJsonResume(true);
          setWorkflowState("PAUSED_EXPIRED");
          break;
        case "paused_retryable":
          setWorkflowState("RESUMABLE");
          break;
        case "completed_with_failures":
          setWorkflowState("PAUSED_ERROR");
          break;
        case "completed":
          setWorkflowState("COMPLETED");
          break;
        default:
          if (selectedJsonName.length > 0) {
            setWorkflowState("IMPORTED");
          } else {
            setWorkflowState("IDLE");
          }
          break;
      }
    };

    const refreshJobState = async (): Promise<void> => {
      const nextJobState = await getJobState();
      if (!isMounted) {
        return;
      }

      applyJobState(nextJobState);
    };

    const unlistenPromise = onDownloadProgress((payload) => {
      if (!isMounted) {
        return;
      }

      if (payload.status === "error" && payload.errorCode === "EXPIRED_LINK") {
        setNeedsJsonResume(true);
        setWorkflowState("PAUSED_EXPIRED");
        setStatusMessage("Upload new JSON to resume");
      }

      if (payload.status === "success") {
        setWorkflowState("RUNNING");
      }

      setProgressPayload(payload);
    });

    void getJobState()
      .then((nextJobState) => {
        if (!isMounted) {
          return;
        }

        applyJobState(nextJobState);

        if (nextJobState.totalFiles > 0) {
          setStatusMessage(
            `Job state: ${nextJobState.status} (${nextJobState.downloadedFiles}/${nextJobState.totalFiles})`,
          );
        }
      })
      .catch(() => {
        if (!isMounted) {
          return;
        }

        setStatusMessage((currentStatus) => currentStatus);
      });

    const interval = window.setInterval(() => {
      void refreshJobState().catch(() => undefined);
    }, 3000);

    return () => {
      isMounted = false;
      window.clearInterval(interval);
      void unlistenPromise.then((unlisten) => {
        unlisten();
      });
    };
  }, [selectedJsonName.length]);

  const progressPercent = useMemo(() => {
    if (progressPayload.totalFiles === 0) {
      return 0;
    }

    return Math.round(
      (progressPayload.completedFiles / progressPayload.totalFiles) * 100,
    );
  }, [progressPayload.completedFiles, progressPayload.totalFiles]);

  async function handleFilesSelected(files: FileList | null): Promise<void> {
    const file = files?.[0];
    if (!file) {
      return;
    }

    setSelectedJsonName(file.name);
    setStatusMessage(`Importing ${file.name}...`);

    try {
      const jsonContent = await file.text();
      const importResult = await importMemoriesJson(jsonContent);

      if (needsJsonResume) {
        setNeedsJsonResume(false);
        setWorkflowState("RESUMABLE");
        setStatusMessage(
          `Selected JSON: ${file.name}. Ready to resume export. Imported ${importResult.importedCount}, skipped ${importResult.skippedDuplicates}.`,
        );
        return;
      }

      setWorkflowState("IMPORTED");
      setStatusMessage(
        `Selected JSON: ${file.name}. Imported ${importResult.importedCount}/${importResult.parsedCount} memories (skipped duplicates: ${importResult.skippedDuplicates}).`,
      );
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : "Unexpected import error.";
      setStatusMessage(`Failed to import JSON: ${errorMessage}`);
    }
  }

  async function startDownload(): Promise<void> {
    if (outputDirectory.trim().length === 0) {
      setStatusMessage("Select an export directory before starting the download.");
      return;
    }

    setIsDownloading(true);
    setWorkflowState("RUNNING");
    setStatusMessage("Downloading queued memories...");

    try {
      const downloadedCount = await downloadQueuedMemories(outputDirectory);

      setStatusMessage(`Download completed. ${downloadedCount} file(s) saved.`);
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : "Unexpected download error.";
      setStatusMessage(`Download failed: ${errorMessage}`);
    } finally {
      setIsDownloading(false);
    }
  }

  async function resumeDownload(): Promise<void> {
    if (outputDirectory.trim().length === 0) {
      setStatusMessage("Select an export directory before resuming download.");
      return;
    }

    setIsDownloading(true);
    setWorkflowState("RUNNING");
    setStatusMessage("Resuming export downloads...");

    try {
      const resumedCount = await resumeExportDownloads(outputDirectory);
      setStatusMessage(`Resume completed. ${resumedCount} file(s) downloaded.`);
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : "Unexpected resume error.";
      setStatusMessage(`Resume failed: ${errorMessage}`);
    } finally {
      setIsDownloading(false);
    }
  }

  async function processMemories(): Promise<void> {
    if (outputDirectory.trim().length === 0) {
      setStatusMessage("Select an export directory before processing files.");
      return;
    }

    setIsProcessing(true);
    setStatusMessage("Processing downloaded memories...");

    try {
      const result = await processDownloadedMemories(outputDirectory, keepOriginals);
      setStatusMessage(
        `Processing completed. Processed ${result.processedCount}, failed ${result.failedCount}.`,
      );
    } catch (error) {
      const errorMessage =
        error instanceof Error ? error.message : "Unexpected processing error.";
      setStatusMessage(`Processing failed: ${errorMessage}`);
    } finally {
      setIsProcessing(false);
    }
  }

  const progressLabel =
    progressPayload.totalFiles > 0
      ? `${progressPayload.completedFiles}/${progressPayload.totalFiles}`
      : "0/0";

  const canResume = workflowState === "RESUMABLE" || workflowState === "PAUSED_ERROR";
  const canStart = workflowState === "IDLE" || workflowState === "IMPORTED" || workflowState === "COMPLETED";

  return (
    <main className="mx-auto flex min-h-screen w-full max-w-3xl items-center px-4 py-8">
      <Card className="w-full">
        <CardHeader>
          <CardTitle>MemorySnaper Dashboard</CardTitle>
          <CardDescription>
            Upload export JSON, choose a destination folder, and track download
            progress in real time.
          </CardDescription>
          <p className="text-xs text-muted-foreground">
            Workflow state: {workflowState}
            {jobState ? ` · Job ${jobState.status} (${jobState.downloadedFiles}/${jobState.totalFiles})` : ""}
          </p>
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
                void handleFilesSelected(event.dataTransfer.files);
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
                  onChange={(event) => void handleFilesSelected(event.target.files)}
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

          <section className="flex items-center justify-between rounded-lg border border-border px-3 py-2 text-sm">
            <label className="flex items-center gap-2">
              <input
                type="checkbox"
                checked={keepOriginals}
                onChange={(event) => setKeepOriginals(event.currentTarget.checked)}
              />
              Keep original downloaded files after processing
            </label>
            <Button
              type="button"
              variant="outline"
              onClick={() => void processMemories()}
              disabled={isProcessing}
            >
              {isProcessing ? "Processing..." : "Process files"}
            </Button>
          </section>

          <section className="flex items-center justify-between gap-3">
            <p className="text-xs text-muted-foreground">{statusMessage}</p>
            <div className="flex items-center gap-2">
              <Button
                type="button"
                variant="outline"
                onClick={() => void resumeDownload()}
                disabled={isDownloading || !canResume}
              >
                Resume export
              </Button>
              <Button
                type="button"
                onClick={() => void startDownload()}
                disabled={isDownloading || !canStart}
              >
                {isDownloading ? "Downloading..." : "Start export"}
              </Button>
            </div>
          </section>
        </CardContent>
      </Card>
    </main>
  );
}

export default App;
