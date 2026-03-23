import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { BrowserRouter } from "react-router-dom";
import { UiStateProvider } from "@/lib/ui-state-provider";
import "./index.css";
import App from "./App.tsx";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <UiStateProvider>
      <BrowserRouter>
        <App />
      </BrowserRouter>
    </UiStateProvider>
  </StrictMode>,
);
