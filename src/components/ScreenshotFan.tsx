import { useRef, type CSSProperties } from "react";

export type Screenshot = { id: number; name: string; thumbnail: string };

export function ScreenshotFan({ shots, german, onDrag }: {
  shots: Screenshot[];
  german: boolean;
  onDrag: (id: number) => void;
}) {
  const origin = useRef<{ x: number; y: number; pointer: number } | null>(null);
  return (
    <aside className="screenshot-fan" aria-label={german ? "Letzte Screenshots" : "Recent screenshots"}>
      {["left", "right"].map((side, parity) => (
        <div className={"fan-wing " + side} key={side}>
          {shots.filter((_, i) => i % 2 === parity).map((shot, layer) => (
            <div className="fan-card" key={shot.id} tabIndex={0}
              style={{ "--layer": layer } as CSSProperties}
              title={shot.name + (german ? " · Zum Einfügen ziehen" : " · Drag to insert")}
              onPointerDown={e => {
                if (!e.isPrimary || e.button !== 0) return;
                e.preventDefault();
                e.currentTarget.focus({ preventScroll: true });
                e.currentTarget.setPointerCapture(e.pointerId);
                origin.current = { x: e.clientX, y: e.clientY, pointer: e.pointerId };
              }}
              onPointerMove={e => {
                const start = origin.current;
                if (!start || start.pointer !== e.pointerId || !(e.buttons & 1)) return;
                if (Math.hypot(e.clientX - start.x, e.clientY - start.y) < 5) return;
                origin.current = null;
                e.currentTarget.releasePointerCapture(e.pointerId);
                onDrag(shot.id);
              }}
              onLostPointerCapture={() => { origin.current = null; }}>
              <img src={shot.thumbnail} alt={shot.name} draggable={false} />
            </div>
          ))}
        </div>
      ))}
    </aside>
  );
}
