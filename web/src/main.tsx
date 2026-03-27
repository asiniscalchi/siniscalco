import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { BrowserRouter } from "react-router-dom";
import { ApolloProvider } from "@apollo/client/react";
import { UiStateProvider } from "@/lib/ui-state-provider";
import { createApolloClient } from "@/lib/apollo";
import "./index.css";
import App from "./App.tsx";

const apolloClient = createApolloClient();

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <ApolloProvider client={apolloClient}>
      <UiStateProvider>
        <BrowserRouter>
          <App />
        </BrowserRouter>
      </UiStateProvider>
    </ApolloProvider>
  </StrictMode>,
);
