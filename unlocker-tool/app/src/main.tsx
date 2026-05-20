import React from "react";
import ReactDOM from "react-dom/client";
import { App } from "./App";
import { HelperGate } from "./screens/HelperGate";
import { UpdateChecker } from "./components/UpdateChecker";
import { loadPlatform } from "./platform";
import "./index.css";

// Resolve target OS before mounting so platform-aware copy renders correctly
// on first paint instead of flashing macOS strings on Windows. Use a .then()
// chain rather than top-level await — the production esbuild target is es2021
// which doesn't permit top-level await.
loadPlatform()
  .catch(() => {
    // Platform invoke failed — fall through to the default ("macos") in
    // platform.ts so the UI renders rather than hanging on a blank page.
  })
  .then(() => {
    ReactDOM.createRoot(document.getElementById("root")!).render(
      <React.StrictMode>
        <HelperGate>
          <App />
        </HelperGate>
        <UpdateChecker />
      </React.StrictMode>,
    );
  });
