import { useState } from "react";
import { Download, Images, Settings, CircleHelp } from "lucide-react";

import { cn } from "@/lib/utils";
import { useI18n } from "@/lib/i18n";
import type { TabKey } from "@/components/AppSidebar";
import { GuideListSheet } from "@/components/GuideListSheet";

interface BottomNavProps {
  activeTab: TabKey;
  onTabChange: (tab: TabKey) => void;
}

const NAV_ITEMS: Array<{
  key: TabKey;
  icon: React.ComponentType<{ className?: string }>;
}> = [
  { key: "downloader", icon: Download },
  { key: "viewer", icon: Images },
  { key: "settings", icon: Settings },
];

export function BottomNav({ activeTab, onTabChange }: BottomNavProps) {
  const { t } = useI18n();
  const [helpOpen, setHelpOpen] = useState(false);

  const labels: Record<TabKey, string> = {
    downloader: t("app.tabs.downloader"),
    viewer: t("app.tabs.viewer"),
    settings: t("app.tabs.settings"),
  };

  return (
    <nav className="fixed bottom-0 left-0 right-0 z-50 border-t bg-background/95 backdrop-blur-sm md:hidden">
      <div role="tablist" className="mx-auto flex h-16 max-w-lg items-center justify-around px-2">
        {NAV_ITEMS.map((item) => {
          const Icon = item.icon;
          const isActive = activeTab === item.key;
          return (
            <button
              key={item.key}
              type="button"
              role="tab"
              aria-selected={isActive}
              onClick={() => onTabChange(item.key)}
              className={cn(
                "flex flex-1 flex-col items-center gap-0.5 rounded-lg py-1.5 text-xs transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2",
                isActive
                  ? "text-primary font-medium"
                  : "text-muted-foreground hover:text-foreground",
              )}
            >
              <div
                className={cn(
                  "flex h-8 w-12 items-center justify-center rounded-full transition-colors",
                  isActive && "bg-primary/10",
                )}
              >
                <Icon className="size-5" />
              </div>
              <span>{labels[item.key]}</span>
            </button>
          );
        })}
        <button
          type="button"
          onClick={() => setHelpOpen(true)}
          className="flex flex-1 flex-col items-center gap-0.5 rounded-lg py-1.5 text-xs text-muted-foreground transition-colors hover:text-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring focus-visible:ring-offset-2"
        >
          <div className="flex h-8 w-12 items-center justify-center rounded-full">
            <CircleHelp className="size-5" />
          </div>
          <span>{t("app.sidebar.help")}</span>
        </button>
      </div>
      <GuideListSheet open={helpOpen} onOpenChange={setHelpOpen} />
    </nav>
  );
}
