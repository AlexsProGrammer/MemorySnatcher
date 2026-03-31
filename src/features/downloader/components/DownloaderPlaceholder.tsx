import { useState } from "react";
import { BookOpen } from "lucide-react";

import { Workflow } from "@/features/downloader/components/Workflow";
import { useI18n } from "@/lib/i18n";
import { Button } from "@/components/ui/button";
import { GuideDialog } from "@/components/GuideDialog";
import { getGuideById } from "@/data/guides/index";

export function DownloaderPlaceholder() {
  const { t } = useI18n();
  const [guideOpen, setGuideOpen] = useState(false);
  const guide = getGuideById("snapchat-export") ?? null;

  return (
    <div className="mx-auto w-full max-w-4xl space-y-6 pb-8 md:pb-10">
      <div className="space-y-1">
        <h2 className="text-lg font-semibold tracking-tight">
          {t("downloader.card.title")}
        </h2>
        <p className="text-sm text-muted-foreground">
          {t("downloader.card.description")}
        </p>
        <Button
          variant="link"
          size="sm"
          className="h-auto gap-1.5 px-0 text-sm"
          onClick={() => setGuideOpen(true)}
        >
          <BookOpen className="size-3.5" />
          {t("downloader.workflow.guideLink")}
        </Button>
      </div>
      <Workflow />
      <GuideDialog guide={guide} open={guideOpen} onOpenChange={setGuideOpen} />
    </div>
  );
}
