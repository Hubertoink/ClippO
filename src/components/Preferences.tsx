import { useState, type ReactNode } from "react";
import { ArrowLeft, FolderOpen, X } from "lucide-react";
import { open } from "@tauri-apps/plugin-dialog";
import { linuxDesktop, native, type Settings } from "../lib/types";
import { Button, Toggle } from "./ui";
type Props = {
  s: Settings;
  set: (v: Partial<Settings>) => void;
  close: () => void;
  initial: string;
};
export function Preferences({ s, set, close, initial }: Props) {
  const [tab, setTab] = useState(initial);
  const t = (en: string, de: string) => (s.language === "de" ? de : en);
  const row = (label: string, el: ReactNode) => (
    <div className="pref-row">
      <span>{label}</span>
      {el}
    </div>
  );
  const toggle = (
    key: "auto" | "watch" | "preserve" | "replace" | "compact" | "underMb",
    label: string,
  ) =>
    row(
      label,
      <Toggle
        checked={s[key]}
        label={label}
        onChange={(v) =>
          set(
            key === "auto" && v
              ? { auto: true, watch: true }
              : key === "watch" && !v
                ? { watch: false, auto: false }
                : { [key]: v },
          )
        }
      />,
    );
  const select = (
    key: "theme" | "motion" | "output" | "language",
    opts: Record<string, string>,
    label: string,
  ) =>
    row(
      label,
      <select
        aria-label={label}
        value={s[key]}
        onChange={(e) => set({ [key]: e.target.value })}
      >
        {Object.entries(opts).map(([v, vLabel]) => (
          <option value={v} key={v}>
            {vLabel}
          </option>
        ))}
      </select>,
    );
  const range = (
    key: "jpg" | "webp" | "pdf" | "png",
    label: string,
    max = 100,
  ) =>
    row(
      label,
      <div className="slider">
        <input
          aria-label={label}
          type="range"
          min={key === "png" ? 0 : 10}
          max={max}
          value={s[key]}
          onChange={(e) => set({ [key]: +e.target.value, mode: "Custom" })}
        />
        <output>{s[key]}</output>
      </div>,
    );
  return (
    <section className="preferences">
      <div className="view-title">
        <Button variant="icon" aria-label={t("Back", "Zurück")} onClick={close}>
          <ArrowLeft />
        </Button>
        <h1>{t("Preferences", "Einstellungen")}</h1>
      </div>
      <div className="tabs">
        {["General", "Compression", "Appearance"].map((v, i) => (
          <button key={v} aria-pressed={tab === v} onClick={() => setTab(v)}>
            {t(v, ["Allgemein", "Komprimierung", "Darstellung"][i])}
          </button>
        ))}
      </div>
      <div className="pref-body">
        {tab === "General" ? (
          <>
            {select(
              "language",
              { en: "English", de: "Deutsch" },
              t("Language", "Sprache"),
            )}
            {native && (
              <>
                {toggle(
                  "watch",
                  t("Detect screenshots", "Screenshots erkennen"),
                )}
                {!linuxDesktop && toggle(
                  "auto",
                  t(
                    "Auto-compress clipboard images",
                    "Zwischenablage automatisch komprimieren",
                  ),
                )}
                {row(
                  t("Auto-save folder", "Auto-Speicherort"),
                  <div className="folder">
                    <button
                      onClick={async () => {
                        const p = await open({ directory: true });
                        if (typeof p === "string") set({ folder: p });
                      }}
                    >
                      <FolderOpen size={14} />
                      {s.folder.split(/[\\/]/).pop() ||
                        t("Choose folder", "Ordner wählen")}
                    </button>
                    {s.folder && (
                      <button
                        onClick={() => set({ folder: "" })}
                        aria-label={t("Clear folder", "Ordner entfernen")}
                      >
                        <X size={14} />
                      </button>
                    )}
                  </div>,
                )}
                {toggle(
                  "replace",
                  t(
                    "Replace originals automatically",
                    "Originale automatisch ersetzen",
                  ),
                )}
              </>
            )}
            {toggle("preserve", t("Preserve metadata", "Metadaten erhalten"))}
          </>
        ) : tab === "Compression" ? (
          <>
            {toggle("underMb", t("Size target: under 1 MB", "Größenziel: unter 1 MB"))}
            {range("jpg", t("JPEG quality", "JPEG-Qualität"))}
            {range("webp", t("WebP quality", "WebP-Qualität"))}
            {range("png", t("PNG optimization", "PNG-Optimierung"), 6)}
            {range("pdf", t("PDF quality", "PDF-Qualität"))}
            {select(
              "output",
              {
                Original: t("Original format", "Originalformat"),
                WebP: "WebP",
                JPEG: "JPEG",
                PNG: "PNG",
              },
              t("Image output", "Bildformat"),
            )}
            {row(
              t("Maximum size (KB)", "Maximale Größe (KB)"),
              <input
                type="number"
                min="1"
                max="256000"
                aria-label="Maximum KB"
                placeholder={t("No limit", "Kein Limit")}
                value={s.maxKb || ""}
                onChange={(e) =>
                  set({
                    maxKb: Math.min(
                      256000,
                      Math.max(0, Number(e.target.value)),
                    ),
                  })
                }
              />,
            )}
          </>
        ) : (
          <>
            {toggle("compact", t("Compact view", "Kompaktansicht"))}
            {select(
              "theme",
              {
                Dark: t("Dark", "Dunkel"),
                Light: t("Light", "Hell"),
                System: "System",
              },
              t("Theme", "Design"),
            )}
            {select(
              "motion",
              {
                Full: t("Full", "Voll"),
                Subtle: t("Subtle", "Dezent"),
                Off: t("Off", "Aus"),
              },
              t("Animation", "Animation"),
            )}
          </>
        )}
      </div>
    </section>
  );
}
