import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import { installDeploymentBaseHistory } from "./routes";
import "./styles.css";

installDeploymentBaseHistory();

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);

