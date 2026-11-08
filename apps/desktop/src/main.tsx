import React from "react";
import ReactDOM from "react-dom/client";
import { App, ErrorBoundary, installProblemLog } from "@plc-lab/ui";
import "@plc-lab/ui/styles.css";

const root = document.getElementById("root");
if (!root) throw new Error("PLC Lab root element is missing.");

installProblemLog();
ReactDOM.createRoot(root).render(
  <React.StrictMode>
    <ErrorBoundary scope="app">
      <App />
    </ErrorBoundary>
  </React.StrictMode>,
);
