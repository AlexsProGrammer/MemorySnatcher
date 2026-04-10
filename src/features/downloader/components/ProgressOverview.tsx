import {
  Files,
  CopySlash,
  AlertTriangle,
  PackageCheck,
  Cog,
} from "lucide-react";

import { Card, CardContent } from "@/components/ui/card";
import { Progress } from "@/components/ui/progress";
import { Badge } from "@/components/ui/badge";
import { useI18n } from "@/lib/i18n";
import type { DownloadErrorCode, ProcessErrorCode } from "@/lib/memories-api";
import type { TranslationKey } from "@/lib/i18n-messages";

type RuntimeProgress = {
  totalFiles: number;
  completedFiles: number;
  successfulFiles: number;
  failedFiles: number;
  status: string;
  errorCode: DownloadErrorCode | ProcessErrorCode | null;
};

type ProgressOverviewProps = {
  progressValue: number;
  totalFiles: number;
  processedFiles: number;
  downloadedFiles: number;
  duplicatesSkipped: number;
  downloadProgress: RuntimeProgress | null;
  processProgress: RuntimeProgress | null;
  isPaused: boolean;
  isStopped: boolean;
  importState: "idle" | "validating" | "running";
};

type StatCardProps = {
  icon: React.ReactNode;
  label: string;
  value: string | number;
  variant?: "default" | "success" | "warning" | "muted";
};

function StatCard({ icon, label, value, variant = "default" }: StatCardProps) {
  const colorMap = {
    default: "text-foreground",
    success: "text-emerald-600 dark:text-emerald-400",
    warning: "text-amber-600 dark:text-amber-400",
    muted: "text-muted-foreground",
  };

  return (
    <Card className="bg-muted/30">
      <CardContent className="flex items-center gap-3 p-3">
        <div className="text-muted-foreground">{icon}</div>
        <div className="min-w-0 flex-1">
          <p className="text-xs text-muted-foreground truncate">{label}</p>
          <p className={`text-sm font-semibold tabular-nums ${colorMap[variant]}`}>
            {value}
          </p>
        </div>
      </CardContent>
    </Card>
  );
}

const statusLabelKeys: Record<string, TranslationKey> = {
  idle: "downloader.progress.status.idle",
  validating: "downloader.progress.status.validating",
  running: "downloader.progress.status.running",
  paused: "downloader.progress.status.paused",
  stopped: "downloader.progress.status.stopped",
};

function statusToBadgeVariant(
  importState: "idle" | "validating" | "running",
  isPaused: boolean,
  isStopped: boolean,
): { label: string; variant: "default" | "secondary" | "destructive" | "outline" } {
  if (isStopped) return { label: "stopped", variant: "destructive" };
  if (isPaused) return { label: "paused", variant: "secondary" };
  if (importState === "running") return { label: "running", variant: "default" };
  if (importState === "validating") return { label: "validating", variant: "outline" };
  return { label: "idle", variant: "outline" };
}

export function ProgressOverview({
  progressValue,
  totalFiles,
  processedFiles,
  downloadedFiles,
  duplicatesSkipped,
  downloadProgress,
  processProgress,
  isPaused,
  isStopped,
  importState,
}: ProgressOverviewProps) {
  const { t } = useI18n();

  // Effective total excludes duplicates — the real number of unique items to process
  const effectiveTotal = Math.max(0, totalFiles - duplicatesSkipped);

  const completedProcessed = processProgress?.successfulFiles ?? processedFiles;
  const completedDownloaded = downloadProgress?.successfulFiles ?? downloadedFiles;
  const failedFiles = (downloadProgress?.failedFiles ?? 0) + (processProgress?.failedFiles ?? 0);

  const sessionBadge = statusToBadgeVariant(importState, isPaused, isStopped);

  return (
    <div className="space-y-4">
      <div className="flex items-center justify-between gap-3">
        <div className="flex items-center gap-3 min-w-0">
          <span className="text-sm font-medium">
            {t("downloader.progress.title")}
          </span>
          <Badge variant={sessionBadge.variant} className="text-xs">
            {t(statusLabelKeys[sessionBadge.label] ?? "downloader.progress.status.idle")}
          </Badge>
        </div>
        <span className="text-sm font-semibold tabular-nums text-muted-foreground">
          {progressValue}%
        </span>
      </div>

      <Progress value={progressValue} className="h-2.5" />

      <div className="grid grid-cols-2 gap-2 sm:grid-cols-4">
        <StatCard
          icon={<Files className="h-4 w-4" />}
          label={t("downloader.progress.stat.total")}
          value={effectiveTotal}
          variant="muted"
        />
        <StatCard
          icon={<Cog className="h-4 w-4" />}
          label={t("downloader.progress.stat.processed")}
          value={`${completedProcessed} / ${effectiveTotal}`}
        />
        <StatCard
          icon={<PackageCheck className="h-4 w-4" />}
          label={t("downloader.progress.stat.downloaded")}
          value={`${completedDownloaded} / ${effectiveTotal}`}
          variant="success"
        />
        <StatCard
          icon={<CopySlash className="h-4 w-4" />}
          label={t("downloader.progress.stat.skipped")}
          value={duplicatesSkipped}
          variant="muted"
        />
      </div>

      {failedFiles > 0 && (
        <div className="flex items-center gap-2 rounded-md bg-destructive/10 px-3 py-2 text-xs text-destructive">
          <AlertTriangle className="h-3.5 w-3.5" />
          {t("downloader.progress.stat.failed", { count: failedFiles })}
        </div>
      )}
    </div>
  );
}
