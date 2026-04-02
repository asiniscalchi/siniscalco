import { useEffect } from "react";

/**
 * Locks body scroll while `isOpen` is true.
 * Restores overflow on cleanup.
 */
export function useBodyScrollLock(isOpen: boolean) {
  useEffect(() => {
    if (!isOpen) return;
    document.body.style.overflow = "hidden";
    return () => {
      document.body.style.overflow = "";
    };
  }, [isOpen]);
}
