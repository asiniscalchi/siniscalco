import { useEffect, useMemo, useState, type ReactNode } from "react";

import {
  HIDE_VALUES_STORAGE_KEY,
  readInitialHideValues,
  UiStateContext,
} from "@/lib/ui-state";

export function UiStateProvider({ children }: { children: ReactNode }) {
  const [hideValues, setHideValues] = useState(readInitialHideValues);

  useEffect(() => {
    window.localStorage.setItem(
      HIDE_VALUES_STORAGE_KEY,
      hideValues ? "true" : "false",
    );
  }, [hideValues]);

  const value = useMemo(
    () => ({
      hideValues,
      toggleHideValues: () => setHideValues((current) => !current),
    }),
    [hideValues],
  );

  return (
    <UiStateContext.Provider value={value}>{children}</UiStateContext.Provider>
  );
}
