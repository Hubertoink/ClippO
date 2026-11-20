import { useEffect, useRef, useState } from "react";
import { motion, MotionConfig, useReducedMotion, animate } from "motion/react";
import {
  ArrowLeft,
  ArrowRight,
  Check,
  CheckCheck,
  ChevronDown,
  ChevronsDownUp,
  Clipboard,
  Copy,
  FileCheck2,
  FileDown,
  FolderOpen,
  HardDrive,
  Keyboard,
  Layers2,
  LoaderCircle,
  Mail,
  Minus,
  MoreHorizontal,
  Paperclip,
  RotateCcw,
  Scale,
  Settings2,
  Scaling,
  FileType2,
  SlidersHorizontal,
  Sparkles,
  Target,
  UsersRound,
  WandSparkles,
  X,
  Zap,
} from "lucide-react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { getCurrentWebviewWindow } from "@tauri-apps/api/webviewWindow";
import { open } from "@tauri-apps/plugin-dialog";
import { Button } from "./components/ui";
import { ResizeDimensions } from "./components/ResizeDimensions";
import { UpdateNotice, useUpdateCheck } from "./components/UpdateNotice";
import { Preferences } from "./components/Preferences";
import {
  estimated,
  extensions,
  imageExtensions,
  native,
  linuxDesktop,
  pretty,
  readSettings,
  type Input,
  type Mode,
  type ResultFile,
  type Settings,
} from "./lib/types";

type DropAction = "compress" | "resize" | "convert";
const isImageName = (name: string) =>
  imageExtensions.includes(name.split(".").pop()?.toLowerCase() || "");
const windowsDrop = native && !linuxDesktop;
type ExplorerWebview = {
  postMessageWithAdditionalObjects?: (message: string, files: File[]) => void;
};
const isWav = (name: string) => /\.wav$/i.test(name);
function hitAction(x: number, y: number): DropAction {
  const action = document
    .elementFromPoint(x, y)
    ?.closest<HTMLElement>("[data-drop-action]")?.dataset.dropAction;
  return action === "resize" || action === "convert" ? action : "compress";
}
const modes: Mode[] = ["Smart", "Balanced", "Maximum", "Custom"];
const icons = {
  Smart: WandSparkles,
  Balanced: Scale,
  Maximum: Zap,
  Custom: SlidersHorizontal,
};
const pathName = (v: string) => v.split(/[\\/]/).pop() || v;
function LiveSize({ value, off }: { value: number; off: boolean }) {
  const ref = useRef<HTMLSpanElement>(null),
    last = useRef(value);
  useEffect(() => {
    const start = last.current;
    last.current = value;
    if (off || start === value) {
      if (ref.current) ref.current.textContent = pretty(value);
      return;
    }
    const control = animate(start, value, {
      duration: 0.2,
      onUpdate: (n) => {
        if (ref.current) ref.current.textContent = pretty(n);
      },
    });
    return () => control.stop();
  }, [value, off]);
  return <span ref={ref}>{pretty(value)}</span>;
}

function Savings({ value, off }: { value: number; off: boolean }) {
  const ref = useRef<HTMLSpanElement>(null);
  useEffect(() => {
    if (!ref.current) return;
    if (off) {
      ref.current.textContent = value + "%";
      return;
    }
    const control = animate(0, value, {
      duration: 0.42,
      onUpdate: (v) => {
        if (ref.current) ref.current.textContent = Math.round(v) + "%";
      },
    });
    return () => control.stop();
  }, [value, off]);
  return <span ref={ref}>{value}%</span>;
}
export default function App() {
  const [s, setSettings] = useState(readSettings);
  const [preferences, setPref] = useState(""),
    [modeMenu, setModeMenu] = useState(false),
    [actions, setActions] = useState(false);
  const [result, setResult] = useState<ResultFile | null>(null),
    [busy, setBusy] = useState(false),
    [drag, setDrag] = useState(false);
  const [batch, setBatch] = useState<{ file: Input; r: ResultFile }[]>([]),
    [measured, setMeasured] = useState(0);
  const [info, setInfo] = useState<{ name: string; size: number } | null>(null),
    [notice, setNotice] = useState(""),
    [detected, setDetected] = useState<Input | null>(null);
  const [waiting, setWaiting] = useState(0),
    [shortcuts, setShortcuts] = useState(false),
    [fault, setFault] = useState(false);
  const [hoverAction, setHoverAction] = useState<DropAction>("compress");
  const [dragKind, setDragKind] = useState<"image" | "wav" | "other">("other");
  const [edit, setEdit] = useState<{
    action: "resize" | "convert";
    files: Input[];
  } | null>(null);
  const updates = useUpdateCheck(native);
  const updateVisible = !!updates.update;
  const [dockOpen, setDockOpen] = useState(false);
  const docked = native && s.teardrop;
  const collapsed = docked && !(dockOpen || drag || busy || preferences || edit || shortcuts || detected);
  const leaveTimer = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);
  const clearLeave = () => clearTimeout(leaveTimer.current);
  const collapse = () => {
    clearLeave();
    setDockOpen(false);
    setDrag(false);
    setPref("");
    setShortcuts(false);
    setDetected(null);
    setModeMenu(false);
    setActions(false);
    setEdit(null);
  };
  const installing = useRef(false);
  const [percent, setPercent] = useState(50);
  const [format, setFormat] = useState<Settings["output"]>("WebP");
  const queue = useRef<{ file: Input; options?: Partial<Settings> }[]>([]),
    locked = useRef(false),
    latest = useRef(s),
    previous = useRef<Input | null>(null);
  const input = useRef<HTMLInputElement>(null),
    noticeTimer = useRef<ReturnType<typeof setTimeout> | null>(null);
  const hideTimer = useRef<ReturnType<typeof setTimeout> | null>(null),
    modeBox = useRef<HTMLDivElement>(null);
  const osReduced = useReducedMotion(),
    off = !!osReduced || s.motion === "Off";
  const t = (en: string, de: string) => (s.language === "de" ? de : en);
  const set = (v: Partial<Settings>) => {
    if (v.teardrop !== undefined) setDockOpen(false);
    setSettings((old) => ({ ...old, ...v }));
  };
  const [noticePath, setNoticePath] = useState<string>();
  const message = (msg: string, error = false, path?: string) => {
    setNoticePath(path);
    setNotice(msg);
    setFault(error);
    if (noticeTimer.current) clearTimeout(noticeTimer.current);
    noticeTimer.current = setTimeout(() => setNotice(""), error ? 12000 : 5000);
  };
  const caught = (e: unknown) =>
    message(e instanceof Error ? e.message : String(e), true);
  const interact = () => {
    if (hideTimer.current) clearTimeout(hideTimer.current);
  };
  useEffect(() => {
    latest.current = s;
    localStorage.setItem("clippo.preferences", JSON.stringify(s));
    document.documentElement.lang = s.language;
    document.documentElement.dataset.theme = s.theme.toLowerCase();
    document.documentElement.dataset.native = String(native);
    if (native) invoke("set_detection", { enabled: s.watch }).catch(caught);
  }, [s]);
  const height = (preferences ? 420 : shortcuts ? 300 : s.compact ? 236 : result && !busy ? 300 : 260) +
    (updateVisible ? 46 : 0);
  useEffect(() => {
    if (updateVisible && latest.current.teardrop) setDockOpen(true);
  }, [updateVisible]);
  useEffect(() => {
    if (native) void invoke("layout_widget", { docked, collapsed, height }).catch(caught);
  }, [docked, collapsed, height]);
  useEffect(() => {
    if (!modeMenu) return;
    const outside = (e: PointerEvent) => {
      if (!modeBox.current?.contains(e.target as Node)) setModeMenu(false);
    };
    document.addEventListener("pointerdown", outside);
    return () => document.removeEventListener("pointerdown", outside);
  }, [modeMenu]);
  async function run(files: Input[], options?: Partial<Settings>) {
    if (installing.current) return;
    if (latest.current.teardrop) setDockOpen(true);
    setEdit(null);
    queue.current.push(...files.map((file) => ({ file, options })));
    setWaiting(queue.current.length);
    if (locked.current) return;
    locked.current = true;
    setBusy(true);
    setBatch([]);
    interact();
    setPref("");
    setShortcuts(false);
    setActions(false);
    setNotice("");
    setDetected(null);
    setResult(null);
    setModeMenu(false);
    try {
      while (queue.current.length) {
        const item = queue.current.shift()!;
        const file = item.file;
        setWaiting(queue.current.length);
        const pref = { ...latest.current, ...item.options };
        setMeasured(0);
        try {
          if (
            file !== "clipboard" &&
            !("captureId" in file && file.captureId) &&
            !extensions.includes(
              file.name.split(".").pop()?.toLowerCase() || "",
            )
          )
            throw new Error(
              t(
                native ? "Choose an image, PDF or WAV file." : "Choose a supported image or PDF.",
                native ? "Wähle ein Bild, eine PDF- oder WAV-Datei." : "Wähle ein unterstütztes Bild oder PDF.",
              ),
            );
          setInfo(
            file === "clipboard"
              ? { name: t("Clipboard", "Zwischenablage"), size: 0 }
              : { name: file.name, size: file.size },
          );
          let r: ResultFile;
          if (
            native &&
            file !== "clipboard" &&
            "captureId" in file &&
            file.captureId
          )
            r = await invoke("compress_capture", {
              id: file.captureId,
              options: pref,
            });
          else if (native)
            r = await invoke(
              file === "clipboard" ? "compress_clipboard" : "compress_file",
              file === "clipboard"
                ? { options: pref }
                : { path: (file as { path: string }).path, options: pref },
            );
          else {
            const { compressBrowser } = await import("./lib/browser");
            r = await compressBrowser(file as File, pref);
          }
          const source =
            file === "clipboard"
              ? { captureId: r.id, path: "", name: r.name, size: r.original }
              : file;
          previous.current = source;
          setBatch((old) => [...old, { file: source, r }]);
          setResult(r);
          if (r.saved) message(r.saved, false, r.savedPath);
        } catch (e) {
          caught(e);
        }
      }
    } finally {
      locked.current = false;
      setBusy(false);
      setInfo(null);
    }
  }
  async function drop(files: Input[], action: DropAction) {
    if (
      action === "compress" ||
      !files.every((f) => f !== "clipboard" && isImageName(f.name))
    )
      return run(files);
    setNotice("");
    setDetected(null);
    setPref("");
    setShortcuts(false);
    setModeMenu(false);
    setActions(false);
    setEdit({ files, action });
    setInfo(null);
    setFormat(
      files.every(
        (f) => f !== "clipboard" && f.name.toLowerCase().endsWith(".webp"),
      )
        ? "PNG"
        : "WebP",
    );
  }
  const paths = async (items: string[], action: DropAction = "compress") => {
    clearLeave();
    if (latest.current.teardrop) setDockOpen(true);
    const files = await Promise.all(
      items.map(async (path) => ({
        path,
        name: pathName(path),
        size: await invoke<number>("file_size", { path }),
      })),
    );
    await drop(files, action);
  };
  const browse = async () => {
    interact();
    try {
      if (native) {
        const p = await open({
          multiple: true,
          filters: [
            { name: t("Images, PDFs and WAV audio", "Bilder, PDFs und WAV-Audio"), extensions },
          ],
        });
        if (p) await paths(Array.isArray(p) ? p : [p]);
      } else input.current?.click();
    } catch (e) {
      caught(e);
    }
  };
  const paste = async () => {
    interact();
    if (native) {
      await run(["clipboard"]);
      return;
    }
    try {
      const items = await navigator.clipboard.read();
      for (const item of items) {
        const mime = item.types.find(
          (v) => v.startsWith("image/") || v === "application/pdf",
        );
        if (mime) {
          const blob = await item.getType(mime);
          await run([
            new File([blob], "Clipboard." + mime.split("/")[1], { type: mime }),
          ]);
          return;
        }
      }
      message(
        t(
          "Copy an image or file first.",
          "Kopiere zuerst ein Bild oder eine Datei.",
        ),
        true,
      );
    } catch {
      message(
        t(
          "Press Ctrl + V here to paste a copied image.",
          "Drücke hier Strg + V, um ein Bild einzufügen.",
        ),
      );
    }
  };
  const eventRef = useRef({
    paths,
    run,
    paste,
    s,
    preferences,
    modeMenu,
    actions,
    shortcuts,
    edit,
  });
  eventRef.current = {
    paths,
    run,
    paste,
    s,
    preferences,
    modeMenu,
    actions,
    shortcuts,
    edit,
  };
  useEffect(() => {
    const clean: Promise<() => void>[] = [];
    const key = (e: KeyboardEvent) => {
      const st = eventRef.current;
      if (e.key === "Escape") {
        if (st.edit) setEdit(null);
        else if (st.modeMenu) setModeMenu(false);
        else if (st.actions) setActions(false);
        else if (st.preferences) setPref("");
        else if (st.shortcuts) setShortcuts(false);
        else if (native && st.s.teardrop) collapse();
        else if (native) void invoke("hide_widget").catch(caught);
        else setDetected(null);
      }
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "o") {
        e.preventDefault();
        if (e.shiftKey) {
          setPref("");
          setShortcuts(false);
        } else void browse();
      }
      if (
        native &&
        e.ctrlKey &&
        e.key.toLowerCase() === "v" &&
        !(e.target instanceof HTMLInputElement)
      ) {
        e.preventDefault();
        void st.paste();
      }
    };
    const onPaste = (e: ClipboardEvent) => {
      if (native || e.target instanceof HTMLInputElement) return;
      const files = Array.from(e.clipboardData?.files || []);
      if (files.length) {
        e.preventDefault();
        void eventRef.current.run(files);
      }
    };
    window.addEventListener("keydown", key);
    window.addEventListener("paste", onPaste);
    if (native) {
      if (windowsDrop) {
        clean.push(
          listen<{ paths: string[]; action: DropAction }>("windows-file-drop", ({ payload }) => {
            void eventRef.current.paths(payload.paths, payload.action).catch(caught);
          }),
          listen("windows-file-drop-error", () => {
            const german = eventRef.current.s.language === "de";
            caught(german ? "Dateipfad nicht verfügbar. Bitte die Datei auswählen." : "File path unavailable. Please browse for the file.");
          }),
        );
      }
      clean.push(
        getCurrentWebviewWindow().onDragDropEvent(async ({ payload }) => {
          if (payload.type === "over" || payload.type === "enter") {
            clearLeave();
            setHoverAction(
              hitAction(
                payload.position.x / devicePixelRatio,
                payload.position.y / devicePixelRatio,
              ),
            );
          }
          if (payload.type === "enter") {
            const paths = payload.paths;
            setDragKind(
              paths.length && paths.every(isWav) ? "wav" :
              paths.length && paths.every(isImageName) ? "image" : "other",
            );
            setDrag(true);
            if (payload.paths[0]) {
              const path = payload.paths[0];
              const size = await invoke<number>("file_size", { path }).catch(
                () => 0,
              );
              setInfo({ name: pathName(path), size });
            }
          }
          if (payload.type === "leave") {
            leaveTimer.current = setTimeout(() => setDrag(false), 160);
            if (!locked.current) setInfo(null);
          }
          if (payload.type === "drop") {
            setDrag(false);
            await eventRef.current
              .paths(
                payload.paths,
                hitAction(
                  payload.position.x / devicePixelRatio,
                  payload.position.y / devicePixelRatio,
                ),
              )
              .catch(caught);
          }
        }),
      );
      clean.push(
        listen<{ path?: string; wasHidden: boolean }>(
          "capture",
          async ({ payload }) => {
            const cfg = eventRef.current.s;
            if (
              !cfg.watch ||
              locked.current ||
              eventRef.current.preferences ||
              eventRef.current.edit
            )
              return;
            const source: Input = payload.path
              ? {
                  path: payload.path,
                  name: pathName(payload.path),
                  size: await invoke<number>("file_size", {
                    path: payload.path,
                  }).catch(() => 0),
                }
              : "clipboard";
            if (source === "clipboard" && cfg.auto) {
              void eventRef.current.run([source]);
              return;
            }
            setDetected(source);
            interact();
            hideTimer.current = setTimeout(() => {
              setDetected(null);
              if (payload.wasHidden && !locked.current)
                void invoke("hide_widget").catch(caught);
            }, 6500);
          },
        ),
      );
      clean.push(
        listen("widget-opened", () => {
          interact();
          if (latest.current.teardrop) setDockOpen(true);
          setDetected(null);
        }),
        listen("collapse-widget", collapse),
      );
      clean.push(
        listen<number>("compression-progress", ({ payload }) =>
          setMeasured(payload),
        ),
      );
      Promise.all(clean)
        .then(() => invoke("widget_ready"))
        .catch(caught);
    }
    return () => {
      window.removeEventListener("keydown", key);
      clearLeave();
      window.removeEventListener("paste", onPaste);
      clean.forEach((p) => p.then((f) => f()));
    };
  }, []);
  async function action(kind: string) {
    if (!result) return;
    setActions(false);
    try {
      if (native) {
        const text = await invoke<string>("result_action", {
          id: result.id,
          action: kind === "save" ? "save-path" : kind,
          folder: s.folder,
          language: s.language,
        });
        if (kind === "save")
          message(t("Saved to ", "Gespeichert in ") + text, false, text);
        else message(text);
        if (kind === "replace") {
          setResult({ ...result, canReplace: false });
          setBatch((items) =>
            items.map((item) =>
              item.r.id === result.id
                ? { ...item, r: { ...item.r, canReplace: false } }
                : item,
            ),
          );
        }
      } else {
        const { download, copyImage } = await import("./lib/browser");
        if (kind === "save") {
          download(result);
          message(t("Download started", "Download gestartet"));
        } else if (kind === "copy") {
          await copyImage(result);
          message(t("Copied to clipboard", "In Zwischenablage kopiert"));
        } else if (kind === "share") {
          const file = new File([result.blob!], result.name, {
            type: result.blob!.type,
          });
          await navigator.share({ files: [file] });
        }
      }
    } catch (e) {
      if (!(e instanceof DOMException && e.name === "AbortError")) caught(e);
    }
  }
  const modeLabel = (m: Mode) =>
    s.language === "de"
      ? {
          Smart: "Smart",
          Balanced: "Ausgewogen",
          Maximum: "Maximal",
          Custom: "Eigene",
        }[m]
      : m;
  const modeDesc = (m: Mode) =>
    ({
      Smart: t("Prioritize visual quality", "Hohe Bildqualität"),
      Balanced: t(
        "Quality and size in balance",
        "Qualität und Größe im Gleichgewicht",
      ),
      Maximum: t("Smallest possible file", "Möglichst kleine Datei"),
      Custom: t("Your compression settings", "Deine Qualitätseinstellungen"),
    })[m];
  const ModeIcon = icons[s.mode];
  const reset = () => {
    setResult(null);
    setBatch([]);
    setInfo(null);
    setActions(false);
    setNotice("");
    setDetected(null);
  };
  const toggleMb = (on: boolean) => {
    set({ underMb: on });
    if (on && result && previous.current && !busy && result.size >= 1000000)
      void run([previous.current], { underMb: true });
  };
  return (
    <MotionConfig reducedMotion={off ? "always" : "user"}>
      <main
        className={
          "desktop " + (s.compact ? "compact " : "") + (off ? "no-motion " : "") +
          (docked ? (collapsed ? "dock-resting" : "dock-expanded") : "") +
          (s.motion === "Subtle" ? " subtle-motion" : "")
        }
        onDragEnter={(e) => {
          if ((native && !windowsDrop) || !e.dataTransfer.types.includes("Files")) return;
          e.preventDefault();
          clearLeave();
          const first = e.dataTransfer.items[0]?.getAsFile();
          if (first) setInfo({ name: first.name, size: first.size });
          const items = Array.from(e.dataTransfer.items).filter(i => i.kind === "file");
          const audio = items.length > 0 && items.every(i => /^audio\/(?:wav|wave|x-wav|vnd\.wave)$/i.test(i.type));
          const images = items.length > 0 && items.every(i => !i.type || i.type.startsWith("image/"));
          setDragKind(audio && windowsDrop ? "wav" : images ? "image" : "other");
          setDrag(true);
        }}
        onDragOver={(e) => {
          if ((native && !windowsDrop) || !e.dataTransfer.types.includes("Files")) return;
          e.preventDefault();
          e.dataTransfer.dropEffect = "copy";
          clearLeave();
          setDrag(true);
          setHoverAction(hitAction(e.clientX, e.clientY));
        }}
        onDragLeave={(e) => {
          if ((native && !windowsDrop) || !e.dataTransfer.types.includes("Files")) return;
          if (e.relatedTarget instanceof Node && e.currentTarget.contains(e.relatedTarget)) return;
          clearLeave();
          leaveTimer.current = setTimeout(() => {
            setDrag(false);
            if (!locked.current) setInfo(null);
          }, 180);
        }}
        onDrop={(e) => {
          if (native && !windowsDrop) return;
          e.preventDefault();
          clearLeave();
          const action = hitAction(e.clientX, e.clientY);
          setDrag(false);
          const files = Array.from(e.dataTransfer.files);
          if (!files.length) return;
          if (docked) setDockOpen(true);
          if (windowsDrop) {
            const host = (window as Window & { chrome?: { webview?: ExplorerWebview } }).chrome?.webview;
            try {
              if (!host?.postMessageWithAdditionalObjects)
                throw new Error(t("Update Microsoft Edge WebView2 or browse for the file.", "Bitte Microsoft Edge WebView2 aktualisieren oder die Datei auswählen."));
              host.postMessageWithAdditionalObjects(JSON.stringify({ clippo_drop: action }), files);
            } catch (error) { caught(error); }
          } else {
            void drop(files, action).catch(caught);
          }
        }}
      >
        <motion.article
          className={"widget " + (preferences ? "settings-open" : "")}
          initial={off ? false : { opacity: 0, y: 10, scale: 0.985 }}
          animate={{ opacity: 1, y: 0, scale: 1 }}
          transition={
            s.motion === "Full"
              ? { type: "spring", stiffness: 330, damping: 28 }
              : { duration: 0.12 }
          }
          onPointerDown={interact}
          onPointerEnter={interact}
        >
          {collapsed && <button className="teardrop-handle"
            aria-label={t("Open ClippO", "ClippO öffnen")}
            title={t("Drop a file or click to open ClippO", "Datei ablegen oder ClippO per Klick öffnen")}
            onClick={() => setDockOpen(true)}><ChevronsDownUp aria-hidden="true" /></button>}
          <header className="titlebar" data-tauri-drag-region={docked ? undefined : true}>
            <div className="brand" data-tauri-drag-region={docked ? undefined : true}>
              <span className="brand-mark">
                <ChevronsDownUp strokeWidth={2.8} />
              </span>
              <strong>ClippO</strong>
            </div>
            <div className="header-actions">
              <Button
                variant="icon"
                aria-label={t("Keyboard shortcuts", "Tastenkürzel")}
                title={t("Keyboard shortcuts", "Tastenkürzel")}
                onClick={() => {
                  setShortcuts(!shortcuts);
                  setModeMenu(false);
                  setPref("");
                }}
              >
                <Keyboard />
              </Button>
              <Button
                variant="icon"
                aria-label={t("Preferences", "Einstellungen")}
                title={t("Preferences", "Einstellungen")}
                aria-pressed={!!preferences}
                onClick={() => {
                  setPref(preferences ? "" : "General");
                  setModeMenu(false);
                  setShortcuts(false);
                }}
              >
                <Settings2 />
              </Button>
              {native && (
                <Button
                  variant="icon"
                  title={docked ? t("Collapse to teardrop", "Zum Tropfen einklappen") : t("Hide widget", "Widget ausblenden")}
                  aria-label={docked ? t("Collapse to teardrop", "Zum Tropfen einklappen") : t("Hide widget", "Widget ausblenden")}
                  onClick={() => docked ? collapse() : invoke("hide_widget").catch(caught)}
                >
                  <Minus />
                </Button>
              )}
            </div>
          </header>
          {updates.update && (
            <UpdateNotice
              update={updates.update}
              portable={updates.portable}
              dismiss={updates.dismiss}
              german={s.language === "de"}
              busy={busy}
              beginInstall={() => {
                if (locked.current || installing.current) return false;
                installing.current = true;
                return true;
              }}
              endInstall={() => { installing.current = false; }}
            />
          )}
          <input
            hidden
            type="file"
            multiple
            ref={input}
            accept={extensions.map((e) => "." + e).join(",")}
            onChange={(e) => {
              if (e.target.files) void run(Array.from(e.target.files));
              e.target.value = "";
            }}
          />
          {preferences ? (
            <Preferences
              s={s}
              set={set}
              initial={preferences}
              updateStatus={updates.status}
              checkUpdates={updates.checkAgain}
              close={() => setPref("")}
            />
          ) : shortcuts ? (
            <section className="preferences">
              <div className="view-title">
                <Button
                  variant="icon"
                  aria-label={t("Back", "Zurück")}
                  onClick={() => setShortcuts(false)}
                >
                  <ArrowLeft />
                </Button>
                <h1>{t("Keyboard shortcuts", "Tastenkürzel")}</h1>
              </div>
              {[
                [t("Show / minimize ClippO", "ClippO anzeigen / minimieren"), "Ctrl + Shift + O"],
                [t("Browse files", "Dateien wählen"), "Ctrl + O"],
                [t("Paste image", "Bild einfügen"), "Ctrl + V"],
                [t("Dismiss", "Schließen"), "Esc"],
              ].map(([label, keys]) => (
                <div className="pref-row" key={keys}>
                  {label}
                  <kbd>{keys}</kbd>
                </div>
              ))}
            </section>
          ) : (
            <>
              <div className="workarea">
                <section
                  className={
                    "dropzone " +
                    (drag ? "dragging " : "") +
                    (result && !busy ? "completed " : "") +
                    (busy ? "processing" : "")
                  }
                >
                  {drag && !busy ? (
                    <div className="drop-palette">
                      <div className="drop-caption">
                        {info?.size && dragKind === "wav" ? pretty(info.size) : info?.size
                          ? pretty(info.size) +
                            " → ≈ " +
                            pretty(estimated(info.size, s))
                          : t(
                              "Drop on an action",
                              "Zum Ausführen hier ablegen",
                            )}
                      </div>
                      <div className="drop-targets">
                        {(
                          [
                            "compress",
                            ...(dragKind === "image" ? ["resize", "convert"] : []),
                          ] as DropAction[]
                        ).map((kind) => {
                          const Icon =
                            kind === "compress"
                              ? ChevronsDownUp
                              : kind === "resize"
                                ? Scaling
                                : FileType2;
                          return (
                            <div
                              key={kind}
                              data-drop-action={kind}
                              className={hoverAction === kind ? "active" : ""}
                            >
                              <Icon />
                              <b>
                                {kind === "compress"
                                  ? dragKind === "wav"
                                    ? t("Convert to MP3", "In MP3 umwandeln")
                                    : t("Compress", "Komprimieren")
                                  : kind === "resize"
                                    ? t("Resize", "Größe ändern")
                                    : t("Convert", "Format ändern")}
                              </b>
                            </div>
                          );
                        })}
                      </div>
                    </div>
                  ) : edit && !busy ? (
                    <form
                      className="image-editor"
                      onSubmit={(e) => {
                        e.preventDefault();
                        void run(
                          edit.files,
                          edit.action === "resize"
                            ? {
                                resizePercent: percent,
                                output: "Original",
                                maxKb: 0,
                                underMb: false,
                                replace: false,
                              }
                            : { output: format, replace: false },
                        );
                      }}
                    >
                      <strong>
                        {edit.action === "resize"
                          ? t("Resize image", "Bildgröße ändern")
                          : t("Convert image", "Bildformat ändern")}
                      </strong>
                      {edit.action === "resize" ? (
                        <>
                          <ResizeDimensions
                            key={edit.files
                              .map((f) => (f === "clipboard" ? "" : f.name))
                              .join("|")}
                            files={edit.files}
                            percent={percent}
                            german={s.language === "de"}
                          />
                          <label>
                            <input
                              aria-label={t("Scale", "Skalierung")}
                              type="range"
                              min={10}
                              max={99}
                              value={percent}
                              onChange={(e) => setPercent(+e.target.value)}
                            />
                            <output>{percent} %</output>
                          </label>
                        </>
                      ) : (
                        <div className="format-picks">
                          {(["WebP", "JPEG", "PNG"] as const).map((f) => (
                            <button
                              type="button"
                              key={f}
                              aria-pressed={f === format}
                              onClick={() => setFormat(f)}
                            >
                              {f}
                            </button>
                          ))}
                        </div>
                      )}
                      <div className="button-row">
                        <Button
                          type="button"
                          variant="secondary"
                          onClick={() => setEdit(null)}
                        >
                          {t("Cancel", "Abbrechen")}
                        </Button>
                        <Button type="submit">{t("Apply", "Anwenden")}</Button>
                      </div>
                    </form>
                  ) : busy ? (
                    <div className="work-state" aria-live="polite">
                      <div className="state-icon">
                        <LoaderCircle className="spin" />
                      </div>
                      <h1>{isWav(info?.name || "") ? t("Converting to MP3…", "Wird in MP3 umgewandelt…") : t("Compressing…", "Wird verarbeitet…")}</h1>
                      <span className="filename">{info?.name}</span>
                      {!!info?.size && (!isWav(info.name) || measured > 0) && (
                        <div className="estimate">
                          {pretty(info.size)} <ArrowRight size={15} />{" "}
                          {measured && busy ? "" : "≈ "}
                          <LiveSize
                            value={
                              measured && busy
                                ? measured
                                : estimated(info.size, s)
                            }
                            off={off}
                          />
                        </div>
                      )}
                      {waiting > 0 && (
                        <small>
                          +{waiting} {t("queued", "in Warteschlange")}
                        </small>
                      )}
                    </div>
                  ) : detected ? (
                    <div className="work-state">
                      <div className="state-icon">
                        <Clipboard />
                      </div>
                      <h1>{t("Screenshot detected", "Screenshot erkannt")}</h1>
                      <div className="button-row">
                        <Button onClick={() => run([detected])}>
                          {t("Compress screenshot", "Screenshot komprimieren")}
                        </Button>
                        <Button
                          variant="icon"
                          aria-label={t("Dismiss", "Schließen")}
                          onClick={() => setDetected(null)}
                        >
                          <X />
                        </Button>
                      </div>
                    </div>
                  ) : result ? (
                    <div className="work-state result" aria-live="polite">
                      <span className="done-icon">
                        <FileCheck2 />
                      </span>
                      {batch.length > 1 ? (
                        <select
                          className="batch-choice"
                          aria-label={t(
                            "Compressed files",
                            "Komprimierte Dateien",
                          )}
                          value={result.id}
                          onChange={(e) => {
                            const next = batch.find(
                              (item) => item.r.id === e.target.value,
                            );
                            if (next) {
                              setResult(next.r);
                              previous.current = next.file;
                            }
                          }}
                        >
                          {batch.map(({ r }, i) => (
                            <option key={r.id} value={r.id}>
                              {i + 1}/{batch.length} · {r.name}
                            </option>
                          ))}
                        </select>
                      ) : (
                        <div className="file-line">{result.name}</div>
                      )}
                      <h1>
                        {result.size > result.original ? (
                          t("File processed", "Datei verarbeitet")
                        ) : (
                          <>
                            <Savings
                              value={Math.round(
                                (1 - result.size / result.original) * 100,
                              )}
                              off={off}
                            />{" "}
                            {t("saved", "gespart")}
                          </>
                        )}
                      </h1>
                      <div className="result-size">
                        {pretty(result.original)} <ArrowRight size={15} />{" "}
                        <b>{pretty(result.size)}</b>
                      </div>
                      {!result.targetMet && (
                        <small>
                          {t(
                            "Size target could not be reached.",
                            "Größenziel konnte nicht erreicht werden.",
                          )}
                        </small>
                      )}
                      <div className="button-row">
                        <Button onClick={() => action("save")}>
                          <FileDown />
                          {t("Save copy", "Kopie speichern")}
                        </Button>
                        {(native || isImageName(result.name)) && (
                          <Button
                            variant="secondary"
                            onClick={() => action(native ? "file" : "copy")}
                          >
                            <Copy />
                            {t("Copy", "Kopieren")}
                          </Button>
                        )}
                        {native && result.canReplace && (
                          <Button
                            variant="secondary"
                            onClick={() => action("replace")}
                          >
                            <RotateCcw />
                            {t("Replace", "Ersetzen")}
                          </Button>
                        )}
                        <Button
                          variant="icon"
                          aria-label={t("Quick actions", "Schnellaktionen")}
                          onClick={() => setActions(!actions)}
                        >
                          <MoreHorizontal />
                        </Button>
                      </div>
                    </div>
                  ) : (
                    <>
                      <div className="hero-art" aria-hidden="true">
                        <span className="orbit orbit-one" />
                        <span className="orbit orbit-two" />
                        <img src="/drop-tray.webp" alt="" draggable={false} />
                        <Sparkles className="twinkle one" />
                        <span className="twinkle two">✦</span>
                      </div>
                      <h1>
                        {t(
                          "Drop a file to compress",
                          "Datei zum Komprimieren ablegen",
                        )}
                      </h1>
                      <p className="hero-sub">
                        {t(
                          native ? "Drop an image, PDF or WAV file." : "Drag in an image or PDF to get started.",
                          native ? "Bild, PDF oder WAV-Datei hier ablegen." : "Zieh ein Bild oder PDF hierher.",
                        )}
                      </p>
                      <div className="button-row">
                        <Button onClick={browse}>
                          <FolderOpen />
                          {t("Browse files", "Dateien wählen")}
                        </Button>
                        <Button variant="secondary" onClick={paste}>
                          <Clipboard />
                          {t("Paste", "Einfügen")}
                          <kbd>Ctrl V</kbd>
                        </Button>
                      </div>
                      <div
                        className="formats"
                        aria-label={t(
                          "Supported formats",
                          "Unterstützte Formate",
                        )}
                      >
                        {["PNG", "JPG", "WEBP", "AVIF", "HEIC", "PDF", ...(native ? ["WAV"] : [])].map(
                          (ext) => (
                            <span key={ext}>{ext}</span>
                          ),
                        )}
                      </div>
                    </>
                  )}
                </section>
                <div
                  className="controls"
                  hidden={!!edit || (s.compact && !!result && !busy)}
                >
                  <div className="mode-control" ref={modeBox}>
                    <button
                      className="mode-select"
                      aria-haspopup="menu"
                      aria-expanded={modeMenu}
                      onClick={() => setModeMenu(!modeMenu)}
                    >
                      <ModeIcon />
                      {modeLabel(s.mode)}
                      <ChevronDown />
                    </button>
                    {modeMenu && (
                      <div className="mode-menu" role="menu">
                        {modes.map((m) => {
                          const Icon = icons[m];
                          return (
                            <button
                              key={m}
                              role="menuitemradio"
                              aria-checked={m === s.mode}
                              onClick={() => {
                                set({ mode: m });
                                setModeMenu(false);
                                if (m === "Custom") setPref("Compression");
                              }}
                            >
                              <Icon />
                              <span>
                                <b>{modeLabel(m)}</b>
                                <small>{modeDesc(m)}</small>
                              </span>
                              {m === s.mode && <Check size={15} />}
                            </button>
                          );
                        })}
                      </div>
                    )}
                  </div>
                </div>
              </div>
              {result && !busy && !edit && (
                <footer>
                  <button className="new-file" onClick={reset}>
                    <Layers2 size={14} />
                    {t("New file", "Neue Datei")}
                  </button>
                </footer>
              )}
            </>
          )}
          {actions && result && (
            <div className="overlay">
              <section className="action-sheet">
                <div className="sheet-title">
                  <strong>{t("Quick actions", "Schnellaktionen")}</strong>
                  <Button
                    variant="icon"
                    aria-label={t("Close", "Schließen")}
                    onClick={() => setActions(false)}
                  >
                    <X />
                  </Button>
                </div>
                {s.compact && result.size >= 1_000_000 && (
                  <button onClick={() => toggleMb(true)}>
                    <Target />
                    {t("Under 1 MB", "Unter 1 MB bringen")}
                  </button>
                )}
                {native ? (
                  <>
                    {[
                      [Copy, t("Copy file", "Datei kopieren"), "file"],
                      ...(isImageName(result.name)
                        ? [
                            [
                              Clipboard,
                              t(
                                "Copy image to clipboard",
                                "Bild in Zwischenablage",
                              ),
                              "copy",
                            ],
                          ]
                        : []),
                      ...(!linuxDesktop ? [[
                        Mail,
                        t("Open Outlook draft", "Outlook-Entwurf öffnen"),
                        "outlook",
                      ]] : []),
                      [
                        Paperclip,
                        t("Prepare attachment", "Anhang vorbereiten"),
                        "attachment",
                      ],
                      ...(!linuxDesktop ? [[
                        UsersRound,
                        t("Copy for Teams", "Für Teams kopieren"),
                        "teams",
                      ]] : []),
                    ].map(([Icon, label, key]) => {
                      const I = Icon as typeof Copy;
                      return (
                        <button
                          key={String(key)}
                          onClick={() => action(String(key))}
                        >
                          <I />
                          {String(label)}
                        </button>
                      );
                    })}
                  </>
                ) : (
                  <>
                    <button onClick={() => action("save")}>
                      <HardDrive />
                      {t("Download file", "Datei herunterladen")}
                    </button>
                    {navigator.canShare?.({
                      files: [
                        new File([result.blob!], result.name, {
                          type: result.blob!.type,
                        }),
                      ],
                    }) && (
                      <button onClick={() => action("share")}>
                        <UsersRound />
                        {t("Share file", "Datei teilen")}
                      </button>
                    )}
                  </>
                )}
                <button
                  onClick={() => {
                    setActions(false);
                    reset();
                  }}
                >
                  <Layers2 />
                  {t("Compress another file", "Weitere Datei komprimieren")}
                </button>
              </section>
            </div>
          )}
          {notice && (
            <div
              className={"notice " + (fault ? "error" : "")}
              role={fault ? "alert" : "status"}
            >
              {fault ? <Sparkles size={15} /> : <CheckCheck size={15} />}
              {noticePath ? (
                <button
                  type="button"
                  className="notice-location"
                  title={t("Show in folder", "Im Ordner anzeigen")}
                  onClick={() =>
                    invoke("reveal_saved", { path: noticePath })
                      .then(() => setNotice(""))
                      .catch(caught)
                  }
                >
                  {notice}
                  <FolderOpen size={15} aria-hidden="true" />
                </button>
              ) : (
                <span>{notice}</span>
              )}
              <button
                aria-label={t("Dismiss", "Schließen")}
                onClick={() => setNotice("")}
              >
                <X size={15} />
              </button>
            </div>
          )}
        </motion.article>
        {!native && (
          <button
            className="shortcut-hint"
            onClick={() => {
              setPref("");
              setShortcuts(!shortcuts);
            }}
          >
            <Keyboard size={15} />
            <span>{t("Open ClippO", "ClippO öffnen")}</span>
            <kbd>Ctrl</kbd>
            <kbd>Shift</kbd>
            <kbd>O</kbd>
          </button>
        )}
      </main>
    </MotionConfig>
  );
}
