import { parseLanguagePreference, type LanguagePreference } from "@/lib/language";

export const SETTINGS_STORAGE_KEY = "memorysnaper.rate-limit-settings";
export const THEME_STORAGE_KEY = "memorysnaper-theme";
export const DOWNLOADER_SESSION_STORAGE_KEY = "memorysnaper.downloader-session.v1";

export type ThemePreference = "light" | "dark" | "system";
export type StartupPagePreference = "system" | "downloader" | "viewer";
export type ThumbnailQualityPreference = "360p" | "480p" | "720p" | "1080p";

export type AppSettings = {
  requestsPerMinute: number;
  concurrentDownloads: number;
  languagePreference: LanguagePreference;
  themePreference: ThemePreference;
  startupPagePreference: StartupPagePreference;
  thumbnailQuality: ThumbnailQualityPreference;
};

const DEFAULT_SETTINGS: AppSettings = {
  requestsPerMinute: 10,
  concurrentDownloads: 3,
  languagePreference: "system",
  themePreference: "system",
  startupPagePreference: "system",
  thumbnailQuality: "480p",
};

export function parseStartupPagePreference(value: string | null): StartupPagePreference {
  if (value === "system" || value === "downloader" || value === "viewer") {
    return value;
  }

  return "system";
}

export function parseThemePreference(value: string | null): ThemePreference {
  if (value === "light" || value === "dark" || value === "system") {
    return value;
  }

  return "system";
}

export function parseThumbnailQualityPreference(
  value: string | null,
): ThumbnailQualityPreference {
  if (value === "360p" || value === "480p" || value === "720p" || value === "1080p") {
    return value;
  }

  return "480p";
}

function normalizeNonNegativeInteger(value: unknown, fallback: number): number {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    return fallback;
  }

  return Math.max(0, Math.floor(value));
}

function parseSettings(rawValue: string): AppSettings | null {
  try {
    const parsedValue: unknown = JSON.parse(rawValue);
    if (!parsedValue || typeof parsedValue !== "object") {
      return null;
    }

    const requestsPerMinute = normalizeNonNegativeInteger(
      Reflect.get(parsedValue, "requestsPerMinute"),
      DEFAULT_SETTINGS.requestsPerMinute,
    );
    const concurrentDownloads = normalizeNonNegativeInteger(
      Reflect.get(parsedValue, "concurrentDownloads"),
      DEFAULT_SETTINGS.concurrentDownloads,
    );
    const languagePreference = parseLanguagePreference(
      typeof Reflect.get(parsedValue, "languagePreference") === "string"
        ? (Reflect.get(parsedValue, "languagePreference") as string)
        : null,
    );
    const themePreference = parseThemePreference(
      typeof Reflect.get(parsedValue, "themePreference") === "string"
        ? (Reflect.get(parsedValue, "themePreference") as string)
        : null,
    );
    const startupPagePreference = parseStartupPagePreference(
      typeof Reflect.get(parsedValue, "startupPagePreference") === "string"
        ? (Reflect.get(parsedValue, "startupPagePreference") as string)
        : null,
    );
    const thumbnailQuality = parseThumbnailQualityPreference(
      typeof Reflect.get(parsedValue, "thumbnailQuality") === "string"
        ? (Reflect.get(parsedValue, "thumbnailQuality") as string)
        : null,
    );

    return {
      requestsPerMinute,
      concurrentDownloads,
      languagePreference,
      themePreference,
      startupPagePreference,
      thumbnailQuality,
    };
  } catch {
    return null;
  }
}

export function readAppSettings(): AppSettings {
  if (typeof window === "undefined") {
    return DEFAULT_SETTINGS;
  }

  const rawValue = window.localStorage.getItem(SETTINGS_STORAGE_KEY);
  if (!rawValue) {
    return DEFAULT_SETTINGS;
  }

  const parsedSettings = parseSettings(rawValue);
  if (!parsedSettings) {
    window.localStorage.removeItem(SETTINGS_STORAGE_KEY);
    return DEFAULT_SETTINGS;
  }

  return parsedSettings;
}

export function writeAppSettings(settings: AppSettings): void {
  if (typeof window === "undefined") {
    return;
  }

  window.localStorage.setItem(SETTINGS_STORAGE_KEY, JSON.stringify(settings));
}

export function clearPersistedAppClientState(): void {
  if (typeof window === "undefined") {
    return;
  }

  window.localStorage.removeItem(SETTINGS_STORAGE_KEY);
  window.localStorage.removeItem(THEME_STORAGE_KEY);
  window.localStorage.removeItem(DOWNLOADER_SESSION_STORAGE_KEY);
}