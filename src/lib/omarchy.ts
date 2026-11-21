import { invoke } from "@tauri-apps/api/core";

type Palette = Record<string, string>;

export function followOmarchyTheme() {
  const root = document.documentElement;
  let applied: Palette = {};
  let active = true;
  let timer: number;

  function apply(next: Palette) {
    for (const key of Object.keys(applied)) {
      if (!(key in next)) root.style.removeProperty("--om-" + key);
    }
    for (const [key, value] of Object.entries(next)) {
      if (applied[key] !== value) root.style.setProperty("--om-" + key, value);
    }
    root.toggleAttribute("data-omarchy", !!next.background);
    applied = next;
  }

  async function refresh() {
    const next = await invoke<Palette | null>("omarchy_palette").catch(() => null);
    if (!active) return;
    apply(next ?? {});
    timer = window.setTimeout(refresh, next ? 3000 : 30000);
  }

  void refresh();
  return () => {
    active = false;
    window.clearTimeout(timer);
    apply({});
  };
}
