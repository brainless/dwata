/**
 * API Configuration
 * In development, uses Vite's proxy (relative URLs)
 * In production, uses window.location.origin
 */

export const API_BASE_URL = import.meta.env.DEV ? "" : (typeof window !== "undefined" ? window.location.origin : "");

/**
 * Helper function to build API URLs
 */
export function getApiUrl(path: string): string {
  // Ensure path starts with /
  const normalizedPath = path.startsWith("/") ? path : `/${path}`;
  return `${API_BASE_URL}${normalizedPath}`;
}
