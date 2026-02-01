/**
 * API Configuration
 * Reads from environment variables set by Vite
 */

const API_HOST = import.meta.env.VITE_API_HOST || "127.0.0.1";
const API_PORT = import.meta.env.VITE_API_PORT || "8080";

export const API_BASE_URL = `http://${API_HOST}:${API_PORT}`;

/**
 * Helper function to build API URLs
 */
export function getApiUrl(path: string): string {
  // Ensure path starts with /
  const normalizedPath = path.startsWith("/") ? path : `/${path}`;
  return `${API_BASE_URL}${normalizedPath}`;
}
