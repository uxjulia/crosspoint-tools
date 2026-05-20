/// <reference types="vite/client" />

interface ImportMetaEnv {
  readonly VITE_SKIP_HELPER?: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
