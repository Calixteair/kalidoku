import js from "@eslint/js";
import ts from "typescript-eslint";
import svelte from "eslint-plugin-svelte";
import svelteParser from "svelte-eslint-parser";
import prettier from "eslint-config-prettier";
import globals from "globals";

export default [
  {
    ignores: [
      "dist/",
      ".astro/",
      "src/paraglide/",
      "node_modules/",
      "playwright-report/",
      "test-results/",
    ],
  },
  js.configs.recommended,
  ...ts.configs.recommended,
  ...svelte.configs["flat/recommended"],
  prettier,
  {
    files: ["**/*.svelte"],
    languageOptions: {
      parser: svelteParser,
      parserOptions: {
        parser: ts.parser,
        extraFileExtensions: [".svelte"],
      },
      globals: { ...globals.browser },
    },
  },
  {
    files: ["public/sw.js"],
    languageOptions: {
      globals: { ...globals.serviceworker, ...globals.browser },
    },
  },
  {
    files: ["tests/e2e/**/*.ts"],
    languageOptions: {
      globals: { ...globals.node },
    },
  },
  {
    rules: {
      "@typescript-eslint/no-unused-vars": [
        "error",
        { argsIgnorePattern: "^_", varsIgnorePattern: "^_" },
      ],
      "no-console": ["warn", { allow: ["warn", "error"] }],
    },
  },
];
