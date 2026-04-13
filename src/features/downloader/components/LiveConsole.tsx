import { useEffect, useRef } from "react";
import { Terminal, Copy } from "lucide-react";

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

function getLineClass(line: string): string {
  if (line.includes("[ERROR]")) return "text-red-400";
  if (
    line.includes("[WARN]") ||
    line.includes("[MISSING]") ||
    line.includes("[SKIP]") ||
    line.includes("[DEBUG]")
  ) return "text-yellow-400";
  if (
    line.includes("[IMPORT]") ||
    line.includes("[DOWNLOAD]") ||
    line.includes("[PROCESS]")
  ) return "text-emerald-400";
  return "text-muted-foreground";
}

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
            <div
              role="button"
              tabIndex={0}
              onClick={(e) => { e.stopPropagation(); onCopyLogs(); }}
              onKeyDown={(e) => { if (e.key === 'Enter' || e.key === ' ') { e.stopPropagation(); e.preventDefault(); onCopyLogs(); } }}
              className={`ml-auto mr-2 inline-flex shrink-0 items-center justify-center gap-1 rounded-md px-2 h-6 text-xs text-muted-foreground hover:bg-accent hover:text-accent-foreground transition-colors ${logLines.length === 0 ? 'pointer-events-none opacity-50' : 'cursor-pointer'}`}
              aria-disabled={logLines.length === 0}
            >
              <Copy className="h-3 w-3" />
              {t("downloader.console.copy")}
            </div>
          </div>
        </AccordionTrigger>
        <AccordionContent className="h-auto px-4 pb-4">
          <div
            ref={scrollRef}
            role="log"
            aria-live="polite"
            className="h-64 overflow-auto rounded-md bg-muted/50 p-3 font-mono text-xs leading-relaxed"
          >
            {logLines.length === 0 ? (
              <p className="italic text-muted-foreground">{t("downloader.console.empty")}</p>
            ) : (
              logLines.map((line, index) => (
                <p key={`${index}-${line.slice(0, 20)}`} className={getLineClass(line)}>{line}</p>
              ))
            )}
          </div>
        </AccordionContent>
      </AccordionItem>
    </Accordion>
  );
}
