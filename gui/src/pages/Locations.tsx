import { createEffect, createSignal, For, Show } from "solid-js";
import type { Location, LocationsResponse } from "../api-types/types";
import { getApiUrl } from "../config/api";

export default function Locations() {
  const [loading, setLoading] = createSignal(true);
  const [error, setError] = createSignal<string | null>(null);
  const [locations, setLocations] = createSignal<Location[]>([]);

  const fetchLocations = async () => {
    setLoading(true);
    setError(null);

    try {
      const response = await fetch(getApiUrl("/api/locations"));

      if (!response.ok) {
        throw new Error(`Failed to fetch locations: ${response.status}`);
      }

      const data: LocationsResponse = await response.json();
      setLocations(data.locations || []);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to fetch locations");
      console.error("Error fetching locations:", err);
    } finally {
      setLoading(false);
    }
  };

  const formatAddress = (loc: Location) => {
    const parts = [loc.address_line1, loc.city, loc.region, loc.postal_code, loc.country_code].filter(Boolean);
    return parts.length > 0 ? parts.join(", ") : loc.name || "-";
  };

  createEffect(() => {
    fetchLocations();
  });

  return (
    <div class="h-full min-h-0 flex flex-col overflow-hidden">
      <header class="pt-4 px-4 md:pt-8 md:px-8 mb-6">
        <h1 class="text-3xl font-bold mb-2">Locations</h1>
        <p class="text-base-content/60">
          Places extracted from your emails
        </p>
      </header>

      <main class="flex-1 min-h-0 overflow-y-auto px-4 md:px-8">
        <Show when={error()}>
          <div class="alert alert-error">
            <div>
              <h3 class="font-bold">Error loading locations</h3>
              <div class="text-sm">{error()}</div>
            </div>
          </div>
        </Show>

        <Show when={loading() && !error()}>
          <div class="flex items-center justify-center py-16">
            <span class="loading loading-spinner loading-lg"></span>
          </div>
        </Show>

        <Show when={!loading() && !error()}>
          <div class="card bg-base-100 shadow-sm border border-base-300">
            <div class="card-body">
              <div class="flex items-center justify-between mb-4">
                <h2 class="card-title">Locations</h2>
                <div class="text-sm text-base-content/60">
                  {locations().length} total
                </div>
              </div>

              <div class="overflow-x-auto">
                <table class="table table-sm">
                  <thead>
                    <tr>
                      <th>Name</th>
                      <th>Address</th>
                    </tr>
                  </thead>
                  <tbody>
                    <Show
                      when={locations().length > 0}
                      fallback={
                        <tr>
                          <td colSpan={2} class="text-center py-8 text-base-content/60">
                            No locations found
                          </td>
                        </tr>
                      }
                    >
                      <For each={locations()}>
                        {(loc) => (
                          <tr class="hover">
                            <td class="font-medium text-sm">{loc.name || "-"}</td>
                            <td class="text-xs text-base-content/70 truncate max-w-[400px]">{formatAddress(loc)}</td>
                          </tr>
                        )}
                      </For>
                    </Show>
                  </tbody>
                </table>
              </div>
            </div>
          </div>
        </Show>
      </main>
    </div>
  );
}
