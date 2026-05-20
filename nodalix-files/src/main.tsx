import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import { perfMark } from "./hooks/usePerformanceMarks";
import "./styles/nodalix.css";

perfMark("bundle_loaded");

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
