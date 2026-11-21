import { isTauri } from "@tauri-apps/api/core";

export const native = isTauri();
export const linuxDesktop = native && /Linux/i.test(navigator.platform);
export const pacmanBuild = linuxDesktop && import.meta.env.VITE_CLIPPO_PACKAGER === "pacman";
export type Mode = "Smart" | "Balanced" | "Maximum" | "Custom";
export type Settings = {
  resizePercent?: number;
  mode: Mode;
  underMb: boolean;
  maxKb: number;
  jpg: number;
  webp: number;
  png: number;
  pdf: number;
  mp3Kbps: number;
  output: "Original" | "WebP" | "JPEG" | "PNG";
  preserve: boolean;
  auto: boolean;
  watch: boolean;
  folder: string;
  replace: boolean;
  compact: boolean;
  teardrop: boolean;
  theme: "Dark" | "Light" | "System";
  motion: "Full" | "Subtle" | "Off";
  language: "en" | "de";
};
export const defaults: Settings = {
  mode: "Balanced",
  underMb: false,
  maxKb: 0,
  jpg: 80,
  webp: 78,
  png: 2,
  pdf: 72,
  mp3Kbps: 192,
  output: "Original",
  preserve: false,
  auto: false,
  watch: true,
  folder: "",
  replace: false,
  compact: false,
  teardrop: false,
  theme: linuxDesktop ? "System" : "Dark",
  motion: "Full",
  language: navigator.language.toLowerCase().split("-")[0] === "de" ? "de" : "en",
};
export function readSettings(): Settings {
  try {
    return {
      ...defaults,
      ...JSON.parse(localStorage.getItem("clippo.preferences") || "{}"),
    };
  } catch {
    return defaults;
  }
}
export type ResultFile = {
  id: string;
  name: string;
  original: number;
  size: number;
  canReplace: boolean;
  targetMet: boolean;
  blob?: Blob;
  source?: File;
  saved?: string;
  savedPath?: string;
};
export type Input =
  | { path: string; name: string; size: number; captureId?: string }
  | File
  | "clipboard";
export function pretty(bytes: number) {
  const units = ["B", "KB", "MB", "GB"];
  const n =
    bytes > 0 ? Math.min(3, Math.floor(Math.log(bytes) / Math.log(1000))) : 0;
  return (
    Number((bytes / 1000 ** n).toFixed(n > 1 ? 2 : n === 1 ? 1 : 0)) +
    " " +
    units[n]
  );
}
export function target(s: Settings) {
  return Math.min(
    s.underMb ? 999999 : Infinity,
    s.maxKb > 0 ? s.maxKb * 1000 : Infinity,
  );
}
export function estimated(size: number, s: Settings) {
  return Math.max(
    1,
    Math.round(
      Math.min(
        target(s),
        size *
          { Smart: 0.63, Balanced: 0.48, Maximum: 0.29, Custom: s.jpg / 125 }[
            s.mode
          ],
      ),
    ),
  );
}
export const imageExtensions = [
  "png",
  "jpg",
  "jpeg",
  "webp",
  "avif",
  "heic",
  "heif",
];
export const extensions = [...imageExtensions, "pdf", ...(native ? ["wav"] : [])];
