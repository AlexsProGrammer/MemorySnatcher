import type { Guide } from "@/data/guides/types";
import type { TabKey } from "@/components/AppSidebar";

import snapchatExport from "@/data/guides/snapchat-export.json";
import extractorUsage from "@/data/guides/extractor-usage.json";
import viewerUsage from "@/data/guides/viewer-usage.json";
import firstTimeSetup from "@/data/guides/first-time-setup.json";

// JSON imports widen string literals; cast is safe because we own the files.
export const guides: Guide[] = [
  firstTimeSetup as Guide,
  snapchatExport as Guide,
  extractorUsage as Guide,
  viewerUsage as Guide,
];

export function getGuideById(id: string): Guide | undefined {
  return guides.find((g) => g.id === id);
}

export function getGuidesForPage(page: TabKey): Guide[] {
  return guides.filter((g) => g.relatedPage === page);
}
