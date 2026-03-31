import { useState, useCallback, useEffect } from "react";
import { ChevronLeft, ChevronRight } from "lucide-react";

import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogDescription,
  DialogFooter,
} from "@/components/ui/dialog";
import { Button } from "@/components/ui/button";
import { useI18n } from "@/lib/i18n";
import type { Guide } from "@/data/guides/types";

interface GuideDialogProps {
  guide: Guide | null;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function GuideDialog({ guide, open, onOpenChange }: GuideDialogProps) {
  const { resolvedLocale, t } = useI18n();
  const [page, setPage] = useState(0);

  // Reset page when dialog opens or guide changes
  useEffect(() => {
    if (open) setPage(0);
  }, [open, guide?.id]);

  const total = guide?.pages.length ?? 0;
  const currentPage = guide?.pages[page];
  const isFirst = page === 0;
  const isLast = page === total - 1;

  const goBack = useCallback(() => setPage((p) => Math.max(0, p - 1)), []);
  const goForward = useCallback(() => {
    if (isLast) {
      onOpenChange(false);
    } else {
      setPage((p) => Math.min(total - 1, p + 1));
    }
  }, [isLast, total, onOpenChange]);

  if (!guide || !currentPage) return null;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>{guide.title[resolvedLocale]}</DialogTitle>
          <DialogDescription className="sr-only">
            {guide.description[resolvedLocale]}
          </DialogDescription>
        </DialogHeader>

        {/* Page image */}
        {currentPage.image && (
          <div className="overflow-hidden rounded-lg border bg-muted">
            <img
              src={currentPage.image}
              alt={currentPage.title[resolvedLocale]}
              className="h-auto w-full object-contain"
            />
          </div>
        )}

        {/* Page content */}
        <div className="space-y-1.5">
          <h3 className="text-sm font-semibold">
            {currentPage.title[resolvedLocale]}
          </h3>
          <p className="text-sm text-muted-foreground leading-relaxed">
            {currentPage.body[resolvedLocale]}
          </p>
        </div>

        {/* Page indicator dots */}
        {total > 1 && (
          <div className="flex items-center justify-center gap-1.5">
            {guide.pages.map((_, i) => (
              <button
                key={i}
                type="button"
                onClick={() => setPage(i)}
                aria-label={`${t("guide.dialog.pageIndicator", {
                  current: i + 1,
                  total,
                })}`}
                className={`size-2 rounded-full transition-colors ${
                  i === page
                    ? "bg-primary"
                    : "bg-muted-foreground/30 hover:bg-muted-foreground/50"
                }`}
              />
            ))}
          </div>
        )}

        <DialogFooter>
          <div className="flex w-full items-center justify-between">
            <span className="text-xs text-muted-foreground">
              {t("guide.dialog.pageIndicator", {
                current: page + 1,
                total,
              })}
            </span>
            <div className="flex gap-2">
              {!isFirst && (
                <Button variant="outline" size="sm" onClick={goBack}>
                  <ChevronLeft data-icon="inline-start" />
                  {t("guide.dialog.prev")}
                </Button>
              )}
              <Button size="sm" onClick={goForward}>
                {isLast ? (
                  t("guide.dialog.done")
                ) : (
                  <>
                    {t("guide.dialog.next")}
                    <ChevronRight data-icon="inline-end" />
                  </>
                )}
              </Button>
            </div>
          </div>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
