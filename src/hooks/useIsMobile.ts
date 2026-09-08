import { useState, useEffect } from "react";
import { api } from "../services/api";

export function useIsMobile() {
  const checkInitialMobile = () => {
    if (typeof window === "undefined") return false;
    const isMobileUA = /Android|webOS|iPhone|iPad|iPod|BlackBerry|IEMobile|Opera Mini/i.test(
      navigator.userAgent
    );
    const isNarrow = window.innerWidth < 768;
    return isMobileUA || isNarrow;
  };

  const [isMobile, setIsMobile] = useState<boolean>(checkInitialMobile);
  const [platform, setPlatform] = useState<string>("unknown");

  useEffect(() => {
    // 1. Query backend platform
    api.getPlatform().then((p) => {
      setPlatform(p);
      if (p === "android") {
        setIsMobile(true);
      }
    });

    // 2. Responsive resize handler
    const handleResize = () => {
      const isMobileUA = /Android|webOS|iPhone|iPad|iPod|BlackBerry|IEMobile|Opera Mini/i.test(
        navigator.userAgent
      );
      const isNarrow = window.innerWidth < 768;
      setIsMobile(isMobileUA || isNarrow || platform === "android");
    };

    window.addEventListener("resize", handleResize);
    return () => window.removeEventListener("resize", handleResize);
  }, [platform]);

  return { isMobile, platform };
}
