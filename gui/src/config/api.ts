/**
 * API Configuration
 * Reads from environment variables set by Vite
 */

const API_HOST = import.meta.env.VITE_API_HOST || "127.0.0.1";
const API_PORT = import.meta.env.VITE_API_PORT || "9200";

const DEFAULT_DEV_API_BASE_URL = `http://${API_HOST}:${API_PORT}`;
const DEFAULT_PROD_API_BASE_URL =
  typeof window !== "undefined" ? window.location.origin : DEFAULT_DEV_API_BASE_URL;

export const API_BASE_URL =
  import.meta.env.VITE_API_BASE_URL ||
  (import.meta.env.DEV ? DEFAULT_DEV_API_BASE_URL : DEFAULT_PROD_API_BASE_URL);

/**
 * Helper function to build API URLs
 */
export function getApiUrl(path: string): string {
  // Ensure path starts with /
  const normalizedPath = path.startsWith("/") ? path : `/${path}`;
  return `${API_BASE_URL}${normalizedPath}`;
}
