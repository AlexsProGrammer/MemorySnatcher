import { useState } from "react";
import {
  Download,
  Images,
  PackageOpen,
  Rocket,
  BookOpen,
  type LucideIcon,
} from "lucide-react";

import {
  Sheet,
  SheetContent,
  SheetHeader,
  SheetTitle,
  SheetDescription,
} from "@/components/ui/sheet";
import { ScrollArea } from "@/components/ui/scroll-area";
import { Separator } from "@/components/ui/separator";
import { GuideDialog } from "@/components/GuideDialog";
import { useI18n } from "@/lib/i18n";
import { guides } from "@/data/guides";
import type { Guide } from "@/data/guides/types";

const iconMap: Record<string, LucideIcon> = {
  Download,
  Images,
  PackageOpen,
  Rocket,
  BookOpen,
};

interface GuideListSheetProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function GuideListSheet({ open, onOpenChange }: GuideListSheetProps) {
  const { resolvedLocale, t } = useI18n();
  const [activeGuide, setActiveGuide] = useState<Guide | null>(null);
  const [guideOpen, setGuideOpen] = useState(false);

  const globalGuides = guides.filter((g) => !g.relatedPage);
  const pageGuides = guides.filter((g) => g.relatedPage);

  function openGuide(guide: Guide) {
    onOpenChange(false);
    setActiveGuide(guide);
    setGuideOpen(true);
  }

  return (
    <>
      <Sheet open={open} onOpenChange={onOpenChange}>
        <SheetContent side="right">
          <SheetHeader>
            <SheetTitle>{t("guide.list.title")}</SheetTitle>
            <SheetDescription className="sr-only">
              {t("guide.list.title")}
            </SheetDescription>
          </SheetHeader>

          <ScrollArea className="flex-1 px-4">
            {/* Getting Started */}
            {globalGuides.length > 0 && (
              <div className="space-y-2">
                <h3 className="text-xs font-medium uppercase tracking-wider text-muted-foreground">
                  {t("guide.list.gettingStarted")}
                </h3>
                {globalGuides.map((guide) => (
                  <GuideCard
                    key={guide.id}
                    guide={guide}
                    locale={resolvedLocale}
                    onClick={() => openGuide(guide)}
                  />
                ))}
              </div>
            )}

            {globalGuides.length > 0 && pageGuides.length > 0 && (
              <Separator className="my-4" />
            )}

            {/* Per-page guides */}
            {pageGuides.length > 0 && (
              <div className="space-y-2">
                <h3 className="text-xs font-medium uppercase tracking-wider text-muted-foreground">
                  {t("guide.list.pageGuides")}
                </h3>
                {pageGuides.map((guide) => (
                  <GuideCard
                    key={guide.id}
                    guide={guide}
                    locale={resolvedLocale}
                    onClick={() => openGuide(guide)}
                  />
                ))}
              </div>
            )}
          </ScrollArea>
        </SheetContent>
      </Sheet>

      <GuideDialog
        guide={activeGuide}
        open={guideOpen}
        onOpenChange={setGuideOpen}
      />
    </>
  );
}

function GuideCard({
  guide,
  locale,
  onClick,
}: {
  guide: Guide;
  locale: "en" | "de";
  onClick: () => void;
}) {
  const Icon = iconMap[guide.icon] ?? BookOpen;

  return (
    <button
      type="button"
      onClick={onClick}
      className="flex w-full items-start gap-3 rounded-lg border p-3 text-left transition-colors hover:bg-muted"
    >
      <Icon className="mt-0.5 size-5 shrink-0 text-primary" />
      <div className="min-w-0 space-y-0.5">
        <p className="text-sm font-medium leading-tight">
          {guide.title[locale]}
        </p>
        <p className="text-xs text-muted-foreground line-clamp-2">
          {guide.description[locale]}
        </p>
      </div>
    </button>
  );
}
