import js from "@eslint/js";
import globals from "globals";
import tseslint from "typescript-eslint";
import reactHooks from "eslint-plugin-react-hooks";
import reactRefresh from "eslint-plugin-react-refresh";

export default tseslint.config(
  {
    ignores: [
      "**/dist/**",
      "target/**",
      ".tools/**",
      "artifacts/**",
      "**/gen/**",
    ],
  },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  {
    files: ["**/*.{ts,tsx}"],
    languageOptions: { globals: { ...globals.browser, ...globals.node } },
    plugins: { "react-hooks": reactHooks, "react-refresh": reactRefresh },
    rules: {
      ...reactHooks.configs.recommended.rules,
      "react-refresh/only-export-components": [
        "error",
        { allowConstantExport: true },
      ],
      "no-restricted-imports": [
        "error",
        {
          paths: [
            {
              name: "@tauri-apps/api/core",
              message:
                "Import the typed @plc-lab/ipc adapter; transport belongs in packages/ipc.",
            },
          ],
        },
      ],
    },
  },
  {
    files: ["packages/ipc/**/*.ts"],
    rules: { "no-restricted-imports": "off" },
  },
  { files: ["**/*.{js,mjs}"], languageOptions: { globals: globals.node } },
  {
    files: ["tests/e2e/*.mjs"],
    languageOptions: { globals: { ...globals.node, ...globals.browser } },
  },
);
