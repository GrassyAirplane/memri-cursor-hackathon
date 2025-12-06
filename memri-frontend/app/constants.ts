// Centralized configuration without relying on .env files.
// These fallbacks mirror the previous defaults.
export const MEMRI_API_URL =
  (typeof process !== "undefined" && process.env.NEXT_PUBLIC_MEMRI_API_URL) ||
  "http://127.0.0.1:8080";

export const MEMRI_API_KEY =
  (typeof process !== "undefined" && process.env.NEXT_PUBLIC_MEMRI_API_KEY) || "";

