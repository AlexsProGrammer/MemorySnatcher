import { CircleHelp } from "lucide-react";

import {
  Tooltip,
  TooltipContent,
  TooltipTrigger,
} from "@/components/ui/tooltip";
import { useI18n } from "@/lib/i18n";
import type { TranslationKey } from "@/lib/i18n-messages";

interface HelpTooltipProps {
  helpKey: TranslationKey;
  className?: string;
}

export function HelpTooltip({ helpKey, className }: HelpTooltipProps) {
  const { t } = useI18n();

  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <button
          type="button"
          className={className}
          aria-label={t(helpKey)}
        >
          <CircleHelp className="size-4 text-muted-foreground" />
        </button>
      </TooltipTrigger>
      <TooltipContent variant="popover" sideOffset={4} className="max-w-xs">
        <p>{t(helpKey)}</p>
      </TooltipContent>
    </Tooltip>
  );
}
