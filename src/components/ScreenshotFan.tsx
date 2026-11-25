import { useRef, type CSSProperties } from "react";
import { linuxDesktop } from "../lib/types";

export type Screenshot = { id: number; name: string; thumbnail: string };
export type CardDragImage = { png: string; x: number; y: number };

// Paint the native cursor image in the card's current pose, including a hover
// transition in progress. The operating system needs a bitmap and a hotspot.
function liftCard(card: HTMLElement, gripX: number, gripY: number): CardDragImage | null {
  const photo = card.querySelector("img");
  if (!photo?.naturalWidth) return null;
  const style = getComputedStyle(card);
  const pose = new DOMMatrixReadOnly(style.transform === "none" ? undefined : style.transform);
  const rect = card.getBoundingClientRect();
  const width = parseFloat(style.width), height = parseFloat(style.height);
  const margin = 8;
  const scale = linuxDesktop ? 1 : window.devicePixelRatio;
  const canvas = document.createElement("canvas");
  canvas.width = Math.ceil((rect.width + margin * 2) * scale);
  canvas.height = Math.ceil((rect.height + margin * 2) * scale);
  const ctx = canvas.getContext("2d");
  if (!ctx) return null;
  ctx.translate(canvas.width / 2, canvas.height / 2);
  ctx.scale(scale, scale);
  ctx.transform(pose.a, pose.b, pose.c, pose.d, 0, 0);
  ctx.translate(-width / 2, -height / 2);
  const grip = ctx.getTransform().transformPoint({ x: gripX, y: gripY });
  ctx.fillStyle = style.backgroundColor;
  ctx.shadowColor = "rgba(32, 27, 25, .25)";
  ctx.shadowBlur = 4 * scale;
  ctx.shadowOffsetY = 2 * scale;
  ctx.beginPath();
  ctx.roundRect(0, 0, width, height, parseFloat(style.borderTopLeftRadius));
  ctx.fill();
  ctx.shadowColor = "transparent";
  const x = parseFloat(style.paddingLeft), y = parseFloat(style.paddingTop);
  const w = width - x - parseFloat(style.paddingRight);
  const h = height - y - parseFloat(style.paddingBottom);
  const cover = Math.max(w / photo.naturalWidth, h / photo.naturalHeight);
  const sw = w / cover, sh = h / cover;
  ctx.drawImage(photo, (photo.naturalWidth - sw) / 2, (photo.naturalHeight - sh) / 2, sw, sh, x, y, w, h);
  return { png: canvas.toDataURL("image/png").split(",")[1], x: Math.round(grip.x), y: Math.round(grip.y) };
}

export function ScreenshotFan({ shots, german, dragging, onOpen, onDrag }: {
  shots: Screenshot[];
  german: boolean;
  dragging: number | null;
  onOpen: (id: number) => void;
  onDrag: (id: number, image: CardDragImage) => void;
}) {
  const origin = useRef<{ x: number; y: number; gripX: number; gripY: number; pointer: number } | null>(null);
  return (
    <aside className="screenshot-fan" aria-label={german ? "Letzte Screenshots" : "Recent screenshots"}>
      {["left", "right"].map((side, parity) => (
        <div className={"fan-wing " + side} key={side}>
          {shots.filter((_, i) => i % 2 === parity).map((shot, layer) => (
            <button type="button" className="fan-card" key={shot.id}
              data-lifted={dragging === shot.id || undefined}
              style={{ "--layer": layer } as CSSProperties}
              title={shot.name + (german ? " · Klicken zum Öffnen, ziehen zum Einfügen" : " · Click to open, drag to insert")}
              onPointerDown={e => {
                if (!e.isPrimary || e.button !== 0) return;
                e.preventDefault();
                e.currentTarget.focus({ preventScroll: true });
                e.currentTarget.setPointerCapture(e.pointerId);
                origin.current = { x: e.clientX, y: e.clientY, gripX: e.nativeEvent.offsetX,
                  gripY: e.nativeEvent.offsetY, pointer: e.pointerId };
              }}
              onPointerMove={e => {
                const start = origin.current;
                if (!start || start.pointer !== e.pointerId || !(e.buttons & 1)) return;
                if (Math.hypot(e.clientX - start.x, e.clientY - start.y) < 5) return;
                origin.current = null;
                e.currentTarget.releasePointerCapture(e.pointerId);
                const image = liftCard(e.currentTarget, start.gripX, start.gripY);
                if (image) onDrag(shot.id, image);
              }}
              onPointerUp={e => {
                const start = origin.current;
                if (!start || start.pointer !== e.pointerId || e.button !== 0) return;
                origin.current = null;
                if (Math.hypot(e.clientX - start.x, e.clientY - start.y) < 5) onOpen(shot.id);
              }}
              onClick={e => { if (e.detail === 0) onOpen(shot.id); }}
              onLostPointerCapture={e => {
                if (origin.current?.pointer === e.pointerId) origin.current = null;
              }}>
              <img src={shot.thumbnail} alt={shot.name} draggable={false} />
            </button>
          ))}
        </div>
      ))}
    </aside>
  );
}
