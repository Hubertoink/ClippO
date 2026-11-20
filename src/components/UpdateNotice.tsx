import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { check, type Update } from "@tauri-apps/plugin-updater";
import { Download, X } from "lucide-react";

import { linuxDesktop } from "../lib/types";

type Props = {
  update: Update;
  portable: boolean;
  dismiss: () => void;
  german: boolean;
  busy: boolean;
  beginInstall: () => boolean;
  endInstall: () => void;
};
type Phase = "available" | "downloading" | "ready" | "installing" | "installed";

export function useUpdateCheck(enabled: boolean) {
  const [request, setRequest] = useState(0);
  const [status, setStatus] = useState<"checking" | "current" | "available" | "error" | "idle">("checking");
  const [update, setUpdate] = useState<Update | null>(null);
  const [portable, setPortable] = useState(false);
  useEffect(() => () => { void update?.close().catch(() => {}); }, [update]);
  useEffect(() => {
    if (!enabled) return;
    let cancelled = false;
    let resource: Update | null = null;
    async function checkVersion() {
      try {
        resource = await check({ timeout: 10000 });
        if (cancelled) {
          await resource?.close();
          return;
        }
        if (!resource) {
          setStatus("current");
          return;
        }
        const isPortable = await invoke<boolean>("portable_build");
        if (cancelled) {
          await resource.close();
          return;
        }
        setPortable(isPortable);
        setUpdate(resource);
        setStatus("available");
      } catch (err) {
        await resource?.close().catch(() => {});
        if (!cancelled) setStatus("error");
        console.warn("ClippO update check:", String(err));
      }
    }
    void checkVersion();
    return () => {
      cancelled = true;
    };
  }, [enabled, request]);
  const checkAgain = () => {
    if (status === "checking" || update) return;
    setStatus("checking");
    setRequest(previous => previous + 1);
  };
  const dismiss = () => {
    setUpdate(null);
    setStatus("idle");
  };
  return { update, portable, status, checkAgain, dismiss };
}

export function UpdateNotice({ update, portable, dismiss, german, busy, beginInstall, endInstall }: Props) {
  const [phase, setPhase] = useState<Phase>("available");
  const [percent, setPercent] = useState<number | null>(null);
  const [error, setError] = useState(false);
  const active = useRef(false);
  const t = (de: string, en: string) => (german ? de : en);
  const working = phase === "downloading" || phase === "installing";
  const advance = async () => {
    if (active.current) return;
    active.current = true;
    setError(false);
    try {
      if (portable) {
        await invoke("open_portable_release");
        return;
      }
      if (phase === "ready") {
        if (!beginInstall()) return;
        setPhase("installing");
        try {
          await update.install();
          if (linuxDesktop) {
            endInstall();
            setPhase("installed");
          }
        } catch (e) {
          endInstall();
          setPhase("available");
          throw e;
        }
      } else {
        let total = 0,
          received = 0;
        setPercent(null);
        setPhase("downloading");
        await update.download(
          (e) => {
            if (e.event === "Started") total = e.data.contentLength ?? 0;
            if (e.event === "Progress") {
              received += e.data.chunkLength;
              if (total)
                setPercent(Math.min(100, Math.round((received * 100) / total)));
            }
          },
          { timeout: 120000 },
        );
        setPhase("ready");
      }
    } catch {
      setError(true);
      setPhase("available");
    } finally {
      active.current = false;
    }
  };
  const label = error
    ? t("Update fehlgeschlagen", "Update failed")
    : phase === "installed"
      ? t("Installiert. ClippO neu starten.", "Installed. Restart ClippO.")
      : phase === "downloading"
      ? t("Wird geladen…", "Downloading…") +
        (percent === null ? "" : " " + percent + " %")
      : phase === "installing"
        ? t("Wird installiert…", "Installing…")
        : phase === "ready"
          ? t("Update bereit", "Update ready")
          : t("Update verfügbar", "Update available");
  const actionLabel = portable
    ? t("Zur ZIP", "Get ZIP")
    : phase === "ready"
      ? t("Installieren", "Install")
      : error
        ? t("Erneut versuchen", "Retry")
        : t("Update laden", "Download");
  return (
    <aside className="update-notice">
      <Download size={16} />
      <span role="status">
        {label} <b>v{update.version}</b>
      </span>
      {!working && phase !== "installed" && (
        <button
          className="update-action"
          disabled={phase === "ready" && busy}
          onClick={advance}
        >
          {actionLabel}
        </button>
      )}
      {!working && (
        <button
          className="update-later"
          aria-label={t("Später", "Later")}
          title={t("Später", "Later")}
          onClick={() => { if (!active.current) dismiss(); }}
        >
          <X size={14} />
        </button>
      )}
    </aside>
  );
}
