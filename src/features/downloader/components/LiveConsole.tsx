import { useEffect, useRef } from "react";
import { Terminal, Copy } from "lucide-react";

import { Button } from "@/components/ui/button";
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from "@/components/ui/accordion";
import { useI18n } from "@/lib/i18n";

type LiveConsoleProps = {
  logLines: string[];
};

export function LiveConsole({ logLines }: LiveConsoleProps) {
  const { t } = useI18n();
  const scrollRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    const el = scrollRef.current;
    if (el) {
      el.scrollTop = el.scrollHeight;
    }
  }, [logLines]);

  const onCopyLogs = () => {
    const text = logLines.join("\n");
    void navigator.clipboard.writeText(text);
  };

  return (
    <Accordion type="single" collapsible defaultValue="console">
      <AccordionItem value="console" className="border rounded-lg">
        <AccordionTrigger className="px-4 py-3 hover:no-underline">
          <div className="flex flex-1 items-center gap-2 text-sm font-medium">
            <Terminal className="h-4 w-4 text-muted-foreground" />
            {t("downloader.console.title")}
            {logLines.length > 0 && (
              <span className="text-xs text-muted-foreground">
                ({logLines.length})
              </span>
            )}
            <Button
              type="button"
              variant="ghost"
              size="sm"
              onClick={(e) => { e.stopPropagation(); onCopyLogs(); }}
              disabled={logLines.length === 0}
              className="ml-auto mr-2 h-6 px-2 text-xs gap-1 text-muted-foreground"
            >
              <Copy className="h-3 w-3" />
              {t("downloader.console.copy")}
            </Button>
          </div>
        </AccordionTrigger>
        <AccordionContent className="h-auto px-4 pb-4">
          <div
            ref={scrollRef}
            role="log"
            aria-live="polite"
            className="h-64 overflow-auto rounded-md bg-muted/50 p-3 font-mono text-xs leading-relaxed text-muted-foreground"
          >
            {logLines.length === 0 ? (
              <p className="italic">{t("downloader.console.empty")}</p>
            ) : (
              logLines.map((line, index) => (
                <p key={`${index}-${line.slice(0, 20)}`}>{line}</p>
              ))
            )}
          </div>
        </AccordionContent>
      </AccordionItem>
    </Accordion>
  );
}
