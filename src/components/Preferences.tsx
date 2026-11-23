import { useEffect, useState, type ReactNode } from "react";
import { ArrowLeft, Check, ChevronsDownUp, FolderOpen, X } from "lucide-react";
import { open } from "@tauri-apps/plugin-dialog";
import { disable, enable, isEnabled } from "@tauri-apps/plugin-autostart";
import type { useUpdateCheck } from "./UpdateNotice";
import { linuxDesktop, native, pacmanBuild, type Settings } from "../lib/types";
import { Button, Toggle } from "./ui";
type Props = {
  s: Settings;
  set: (v: Partial<Settings>) => void;
  close: () => void;
  initial: string;
  updateStatus: ReturnType<typeof useUpdateCheck>["status"];
  checkUpdates: () => void;
};
export function Preferences({ s, set, close, initial, updateStatus, checkUpdates }: Props) {
  const [tab, setTab] = useState(initial);
  const [autostart, setAutostart] = useState<boolean | null>(null);
  const [saving, setSaving] = useState(false);
  const [autostartError, setAutostartError] = useState(false);
  useEffect(() => {
    if (!native) return;
    let mounted = true;
    isEnabled().then(value => {
      if (mounted) setAutostart(value);
    }).catch(() => {
      if (mounted) setAutostartError(true);
    });
    return () => { mounted = false; };
  }, []);
  async function changeAutostart(value: boolean) {
    setSaving(true);
    setAutostartError(false);
    try {
      await (value ? enable() : disable());
      setAutostart(value);
    } catch {
      setAutostartError(true);
    } finally {
      setSaving(false);
    }
  }
  const t = (en: string, de: string) => (s.language === "de" ? de : en);
  const launchLabel = t("Start ClippO at login", "ClippO bei Anmeldung starten");
  const currentLabel = t("ClippO is up to date.", "ClippO ist auf dem neuesten Stand.");
  const row = (label: string, el: ReactNode) => (
    <div className="pref-row">
      <span>{label}</span>
      {el}
    </div>
  );
  const toggle = (
    key: "auto" | "watch" | "preserve" | "replace" | "compact" | "underMb" | "teardrop" | "screenshotFan",
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
        <div className="brand">
          <span className="brand-mark" aria-hidden="true">
            <ChevronsDownUp strokeWidth={2.8} />
          </span>
          <strong>ClippO</strong>
        </div>
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
                {row(launchLabel, <Toggle label={launchLabel} checked={autostart ?? false}
                  disabled={autostart === null || saving} onChange={changeAutostart} />)}
                {autostartError && <p role="alert">{t("Couldn’t access the autostart setting.", "Zugriff auf die Autostart-Einstellung fehlgeschlagen.")}</p>}
                {row(t("App updates", "App-Updates"),
                  pacmanBuild ? <span>{t("Arch package", "Arch-Paket")}</span> :
                  <div className="pref-update">
                    <span role="status">
                      {updateStatus === "current" && (
                        <span title={currentLabel}>
                          <Check size={15} aria-hidden="true" />
                          <span className="sr-only">{currentLabel}</span>
                        </span>
                      )}
                    </span>
                    <button disabled={updateStatus === "checking" || updateStatus === "available"} onClick={checkUpdates}>
                      {updateStatus === "checking" ? t("Checking…", "Suche läuft…") : t("Check for updates", "Nach Updates suchen")}
                    </button>
                  </div>)}
                {updateStatus === "error" && <p role="alert">{t("Update check failed. Please try again.", "Updatesuche fehlgeschlagen. Bitte erneut versuchen.")}</p>}
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
            {native &&
              row(
                t("Custom MP3 bitrate", "Eigene MP3-Bitrate"),
                <select
                  aria-label={t("Custom MP3 bitrate", "Eigene MP3-Bitrate")}
                  value={s.mp3Kbps}
                  onChange={(e) => set({ mp3Kbps: Number(e.target.value), mode: "Custom" })}
                >
                  {[96, 128, 160, 192, 256, 320].map((rate) => (
                    <option key={rate} value={rate}>{rate} kbit/s</option>
                  ))}
                </select>,
              )}
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
            {native && toggle("teardrop", t("Top-edge teardrop", "Tropfen am oberen Bildschirmrand"))}
            {native && toggle("screenshotFan", t("Screenshot fan in the teardrop", "Screenshot-Fächer im Tropfen"))}
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
