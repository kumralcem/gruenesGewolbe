import "./styles.css";

import { mountApp } from "./app";
import { createTauriAdapter } from "./tauri-adapter";

const root = document.querySelector<HTMLElement>("#app");
if (!root) throw new Error("application root is missing");

const adapter = window.__GG_TEST_ADAPTER__ ?? createTauriAdapter();
void mountApp(root, adapter);
