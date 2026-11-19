import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { ArrowRight } from "lucide-react";
import type { Input } from "../lib/types";

type Dimensions = [number, number];
export function ResizeDimensions({
  files,
  percent,
  german,
}: {
  files: Input[];
  percent: number;
  german: boolean;
}) {
  const [index, setIndex] = useState(0);
  const file = files[index] ?? files[0];
  const [preview, setPreview] = useState<{
    file: Input;
    dimensions: Dimensions | null;
  }>();
  useEffect(() => {
    let active = true;
    if (file === "clipboard") return;
    const size =
      file instanceof File
        ? import("../lib/browser").then((m) => m.imageDimensions(file))
        : invoke<Dimensions>("image_dimensions", { path: file.path });
    size
      .then((dimensions) => {
        if (active) setPreview({ file, dimensions });
      })
      .catch(() => {
        if (active) setPreview({ file, dimensions: null });
      });
    return () => {
      active = false;
    };
  }, [file]);
  const ready = preview?.file === file;
  const dims = ready ? preview?.dimensions : null;
  return (
    <div className="resize-preview">
      {files.length > 1 && (
        <select
          className="batch-choice"
          aria-label={german ? "Bild für Vorschau" : "Preview image"}
          value={index}
          onChange={(e) => setIndex(+e.target.value)}
        >
          {files.map((f, i) => (
            <option key={i} value={i}>
              {i + 1}. {f === "clipboard" ? "Clipboard" : f.name}
            </option>
          ))}
        </select>
      )}
      {dims ? (
        <div className="dimension-pair" aria-live="polite">
          <div>
            <small>{german ? "Vorher" : "Before"}</small>
            <b>
              {dims[0]} × {dims[1]} px
            </b>
          </div>
          <ArrowRight size={15} aria-hidden="true" />
          <div>
            <small>{german ? "Nachher" : "After"}</small>
            <b>
              {dims
                .map((n) => Math.max(1, Math.round((n * percent) / 100)))
                .join(" × ")}{" "}
              px
            </b>
          </div>
        </div>
      ) : (
        <small>
          {ready
            ? german
              ? "Pixelmaße nicht verfügbar"
              : "Dimensions unavailable"
            : german
              ? "Pixelmaße werden gelesen…"
              : "Reading dimensions…"}
        </small>
      )}
    </div>
  );
}
