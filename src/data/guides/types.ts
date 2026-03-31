import type { ResolvedLocale } from "@/lib/language";

/** Inline locale text — one value per supported language. */
export type LocaleText = Record<ResolvedLocale, string>;

/** A single page/step within a guide. */
export interface GuidePage {
  /** Optional image path relative to /public, e.g. "/tutorials/snapchat-export/step1.webp" */
  image?: string;
  title: LocaleText;
  body: LocaleText;
}

/** A complete step-by-step guide. */
export interface Guide {
  /** Unique identifier, matches the JSON filename (e.g. "snapchat-export"). */
  id: string;
  /** Lucide icon name rendered in the guide list (e.g. "Download", "Images"). */
  icon: string;
  title: LocaleText;
  description: LocaleText;
  /** If set, this guide appears as contextual help on that tab. */
  relatedPage?: "downloader" | "viewer" | "settings";
  pages: GuidePage[];
}
