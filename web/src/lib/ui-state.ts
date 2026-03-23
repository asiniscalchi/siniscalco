import { createContext, useContext } from "react";

const HIDE_VALUES_STORAGE_KEY = "ui.hide_values";

type UiStateValue = {
  hideValues: boolean;
  toggleHideValues: () => void;
};

const UiStateContext = createContext<UiStateValue | undefined>(undefined);

function readInitialHideValues() {
  if (typeof window === "undefined") {
    return false;
  }

  return window.localStorage.getItem(HIDE_VALUES_STORAGE_KEY) === "true";
}

function useUiState() {
  const context = useContext(UiStateContext);

  if (!context) {
    throw new Error("useUiState must be used within UiStateProvider");
  }

  return context;
}

export {
  HIDE_VALUES_STORAGE_KEY,
  readInitialHideValues,
  UiStateContext,
  useUiState,
};
