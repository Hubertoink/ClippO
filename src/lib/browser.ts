import { target, type ResultFile, type Settings } from "./types";

async function pixels(file: Blob) {
  const url = URL.createObjectURL(file);
  try {
    const img = new Image();
    img.src = url;
    await img.decode();
    return img;
  } finally {
    URL.revokeObjectURL(url);
  }
}
async function readImage(file: File) {
  const ext = file.name.split(".").pop()?.toLowerCase() || "";
  let source: Blob = file;
  if (["heif", "heic"].includes(ext)) {
    const heic = (await import("heic2any")).default;
    const decoded = await heic({ blob: file, toType: "image/png" });
    source = Array.isArray(decoded) ? decoded[0] : decoded;
  }
  return pixels(source);
}
export async function imageDimensions(file: File): Promise<[number, number]> {
  const img = await readImage(file);
  return [img.naturalWidth, img.naturalHeight];
}
async function encode(
  canvas: HTMLCanvasElement,
  mime: string,
  quality: number,
  level: number,
) {
  if (mime === "image/png") {
    const UPNG = (await import("upng-js")).default;
    const data = canvas
      .getContext("2d")!
      .getImageData(0, 0, canvas.width, canvas.height);
    return new Blob(
      [UPNG.encode([data.data.buffer], canvas.width, canvas.height, level)],
      { type: mime },
    );
  }
  return new Promise<Blob>((resolve, reject) =>
    canvas.toBlob(
      (blob) => (blob ? resolve(blob) : reject(new Error("Encoding failed"))),
      mime,
      quality,
    ),
  );
}
export async function compressBrowser(
  file: File,
  s: Settings,
): Promise<ResultFile> {
  if (file.size > 256 * 1024 * 1024)
    throw new Error("File exceeds the 256 MB input limit.");
  const ext = file.name.split(".").pop()?.toLowerCase() || "png";
  let best: Blob = file;
  let suffix = ext;
  if (ext === "pdf") {
    const { PDFDocument, PDFName, PDFRef } = await import("pdf-lib");
    const doc = await PDFDocument.load(await file.arrayBuffer(), {
      updateMetadata: false,
    });
    if (!s.preserve) {
      const info = doc.context.trailerInfo.Info;
      if (info instanceof PDFRef) doc.context.delete(info);
      doc.context.trailerInfo.Info = undefined;
      const meta = doc.catalog.get(PDFName.of("Metadata"));
      if (meta instanceof PDFRef) doc.context.delete(meta);
      doc.catalog.delete(PDFName.of("Metadata"));
    }
    const packed = await doc.save({
      useObjectStreams: true,
      addDefaultPage: false,
    });
    if (packed.length < file.size)
      best = new Blob([new Uint8Array(packed)], { type: "application/pdf" });
  } else {
    if (s.preserve)
      throw new Error(
        s.language === "de"
          ? "Metadaten-Erhalt ist in der Windows-App verfügbar."
          : "Use the Windows app to preserve image metadata.",
      );
    const image = await readImage(file);
    const fmt =
      s.output === "Original"
        ? { jpg: "jpeg", jpeg: "jpeg", png: "png", webp: "webp" }[ext] || "webp"
        : s.output.toLowerCase();
    const wanted =
      s.output !== "Original" &&
      !(fmt === ext || (fmt === "jpeg" && ext === "jpg"));
    const quality =
      s.mode === "Custom"
        ? fmt === "webp"
          ? s.webp
          : s.jpg
        : { Smart: 88, Balanced: 78, Maximum: 49 }[s.mode];
    let scale = (s.resizePercent ?? 100) / 100;
    for (let i = 0; i < 9; i++) {
      const canvas = document.createElement("canvas");
      canvas.width = Math.max(1, Math.round(image.naturalWidth * scale));
      canvas.height = Math.max(1, Math.round(image.naturalHeight * scale));
      const ctx = canvas.getContext("2d")!;
      if (fmt === "jpeg") {
        ctx.fillStyle = "#fff";
        ctx.fillRect(0, 0, canvas.width, canvas.height);
      }
      ctx.drawImage(image, 0, 0, canvas.width, canvas.height);
      const colors =
        s.mode === "Custom"
          ? [0, 256, 192, 128, 96, 64, 32][s.png]
          : s.mode === "Smart"
            ? 256
            : s.mode === "Maximum"
              ? 64
              : 192;
      const candidate = await encode(
        canvas,
        "image/" + fmt,
        Math.max(0.22, (quality - i * 8) / 100),
        colors,
      );
      if (
        candidate.size < best.size ||
        (i === 0 && (wanted || s.resizePercent !== undefined))
      ) {
        best = candidate;
        suffix = fmt === "jpeg" ? "jpg" : fmt;
      }
      if (best.size <= target(s)) break;
      scale *= Math.min(
        0.85,
        Math.max(0.4, Math.sqrt(target(s) / best.size) * 0.96),
      );
    }
  }
  return {
    id: crypto.randomUUID(),
    name: file.name.replace(/\.[^.]+$/, "") + "-clippo." + suffix,
    original: file.size,
    size: best.size,
    canReplace: false,
    targetMet: best.size <= target(s),
    blob: best,
    source: file,
  };
}
export function download(r: ResultFile) {
  const url = URL.createObjectURL(r.blob!);
  const a = document.createElement("a");
  a.href = url;
  a.download = r.name;
  a.click();
  setTimeout(() => URL.revokeObjectURL(url), 30000);
}
export async function copyImage(r: ResultFile) {
  const img = await pixels(r.blob!);
  const canvas = document.createElement("canvas");
  canvas.width = img.width;
  canvas.height = img.height;
  canvas.getContext("2d")!.drawImage(img, 0, 0);
  const png = await new Promise<Blob>((resolve, reject) =>
    canvas.toBlob((b) =>
      b ? resolve(b) : reject(new Error("Cannot copy image")),
    ),
  );
  await navigator.clipboard.write([new ClipboardItem({ "image/png": png })]);
}
