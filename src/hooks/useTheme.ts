import { useEffect, useState } from "react";

type ThemeMode = "system" | "light" | "dark";

const STORAGE_KEY = "editclass.theme";

export function useTheme() {
  const [mode, setMode] = useState<ThemeMode>(() => {
    return (localStorage.getItem(STORAGE_KEY) as ThemeMode) || "system";
  });

  useEffect(() => {
    const root = document.documentElement;
    const apply = (m: ThemeMode) => {
      if (m === "system") {
        root.removeAttribute("data-theme");
      } else {
        root.setAttribute("data-theme", m);
      }
    };
    apply(mode);
    localStorage.setItem(STORAGE_KEY, mode);

    if (mode === "system") {
      const mq = window.matchMedia("(prefers-color-scheme: dark)");
      const handler = () => root.removeAttribute("data-theme");
      mq.addEventListener("change", handler);
      return () => mq.removeEventListener("change", handler);
    }
  }, [mode]);

  return { mode, setMode };
}
