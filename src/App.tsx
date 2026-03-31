import { useEffect, useMemo, useState, useCallback } from "react";

import { TooltipProvider } from "@/components/ui/tooltip";
import { SidebarProvider, SidebarInset } from "@/components/ui/sidebar";
import { Skeleton } from "@/components/ui/skeleton";
import { AppSidebar, type TabKey } from "@/components/AppSidebar";
import { BottomNav } from "@/components/BottomNav";
import { DownloaderPlaceholder } from "@/features/downloader/components/DownloaderPlaceholder";
import { SettingsPlaceholder } from "@/features/settings/components/SettingsPlaceholder";
import { ViewerPlaceholder } from "@/features/viewer/components/ViewerPlaceholder";
import { applyAccentColor, readAppSettings } from "@/lib/app-settings";
import { useI18n } from "@/lib/i18n";
import { getViewerItems } from "@/lib/memories-api";
import { useIsMobile } from "@/hooks/use-mobile";
import { GuideDialog } from "@/components/GuideDialog";
import { getGuideById } from "@/data/guides/index";

function AppSkeleton() {
  return (
    <div className="flex h-screen w-full bg-background">
      {/* Sidebar skeleton — desktop only */}
      <div className="hidden md:flex w-16 flex-col gap-4 border-r p-3">
        <Skeleton className="h-8 w-8 rounded-lg" />
        <div className="mt-4 space-y-3">
          <Skeleton className="h-8 w-8 rounded-md" />
          <Skeleton className="h-8 w-8 rounded-md" />
          <Skeleton className="h-8 w-8 rounded-md" />
        </div>
      </div>
      {/* Content skeleton */}
      <div className="flex-1 p-6 space-y-4">
        <Skeleton className="h-8 w-48" />
        <Skeleton className="h-4 w-72" />
        <div className="mt-8 grid grid-cols-2 sm:grid-cols-3 gap-4">
          <Skeleton className="aspect-9/16 rounded-lg" />
          <Skeleton className="aspect-9/16 rounded-lg" />
          <Skeleton className="aspect-9/16 rounded-lg hidden sm:block" />
        </div>
      </div>
    </div>
  );
}

function App() {
  const [activeTab, setActiveTab] = useState<TabKey | null>(null);
  const { t } = useI18n();
  const isMobile = useIsMobile();
  const [onboardingOpen, setOnboardingOpen] = useState(false);
  const onboardingGuide = getGuideById("first-time-setup") ?? null;

  const handleOnboardingChange = useCallback((open: boolean) => {
    setOnboardingOpen(open);
    if (!open) {
      localStorage.setItem("onboarding-complete", "1");
    }
  }, []);

  useEffect(() => {
    const settings = readAppSettings();
    applyAccentColor(settings.accentColor);

    if (settings.startupPagePreference === "downloader") {
      setActiveTab("downloader");
      return;
    }
    if (settings.startupPagePreference === "viewer") {
      setActiveTab("viewer");
      return;
    }

    getViewerItems(0, 1)
      .then((items) => {
        setActiveTab(items.length > 0 ? "viewer" : "downloader");
      })
      .catch(() => {
        setActiveTab("downloader");
      });
  }, []);

  useEffect(() => {
    if (activeTab !== null && !localStorage.getItem("onboarding-complete")) {
      setOnboardingOpen(true);
    }
  }, [activeTab]);

  const tabContent = useMemo(() => {
    switch (activeTab) {
      case "downloader":
        return {
          title: t("app.section.downloader"),
          component: <DownloaderPlaceholder />,
        };
      case "viewer":
        return {
          title: t("app.section.viewer"),
          component: <ViewerPlaceholder />,
        };
      case "settings":
        return {
          title: t("app.section.settings"),
          component: <SettingsPlaceholder />,
        };
      default:
        return null;
    }
  }, [activeTab, t]);

  if (activeTab === null || tabContent === null) {
    return <AppSkeleton />;
  }

  return (
    <TooltipProvider>
      <SidebarProvider>
        {/* Desktop sidebar — hidden on mobile via the sidebar's built-in responsive behavior */}
        {!isMobile && <AppSidebar activeTab={activeTab} onTabChange={setActiveTab} />}

        <SidebarInset>
          <div className="flex h-screen flex-col">
            {/* Scrollable content */}
            <div className="flex-1 overflow-y-auto pb-20 md:pb-0">
              <main className="mx-auto flex h-full w-full max-w-7xl flex-col px-4 py-6 sm:px-6 lg:px-8">
                <section
                  key={activeTab}
                  aria-label={tabContent.title}
                  className="flex-1 min-h-0 animate-in fade-in duration-200"
                >
                  {tabContent.component}
                </section>
              </main>
            </div>
          </div>
        </SidebarInset>

        {/* Mobile bottom nav */}
        {isMobile && <BottomNav activeTab={activeTab} onTabChange={setActiveTab} />}
      </SidebarProvider>

      <GuideDialog guide={onboardingGuide} open={onboardingOpen} onOpenChange={handleOnboardingChange} />
    </TooltipProvider>
  );
}

export default App;
