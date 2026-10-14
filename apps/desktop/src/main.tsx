import React from "react";
import ReactDOM from "react-dom/client";
import { LabShell } from "@plc-lab/ui";
import "@plc-lab/ui/styles.css";

const root = document.getElementById("root");
if (!root) throw new Error("PLC Lab root element is missing.");

ReactDOM.createRoot(root).render(
  <React.StrictMode>
    <LabShell />
  </React.StrictMode>,
);
