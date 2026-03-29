import { createEffect, createSignal, For, Show } from "solid-js";
import type { Organisation, OrganisationsResponse } from "../api-types/types";
import { getApiUrl } from "../config/api";

export default function Organisations() {
  const [loading, setLoading] = createSignal(true);
  const [error, setError] = createSignal<string | null>(null);
  const [organisations, setOrganisations] = createSignal<Organisation[]>([]);

  const fetchOrganisations = async () => {
    setLoading(true);
    setError(null);

    try {
      const response = await fetch(getApiUrl("/api/organisations"));

      if (!response.ok) {
        throw new Error(`Failed to fetch organisations: ${response.status}`);
      }

      const data: OrganisationsResponse = await response.json();
      setOrganisations(data.organisations || []);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to fetch organisations");
      console.error("Error fetching organisations:", err);
    } finally {
      setLoading(false);
    }
  };

  createEffect(() => {
    fetchOrganisations();
  });

  return (
    <div class="h-full min-h-0 flex flex-col overflow-hidden">
      <header class="pt-4 px-4 md:pt-8 md:px-8 mb-6">
        <h1 class="text-3xl font-bold mb-2">Organisations</h1>
        <p class="text-base-content/60">
          Companies and businesses from your emails
        </p>
      </header>

      <main class="flex-1 min-h-0 overflow-y-auto px-4 md:px-8">
        <Show when={error()}>
          <div class="alert alert-error">
            <div>
              <h3 class="font-bold">Error loading organisations</h3>
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
                <h2 class="card-title">Organisations</h2>
                <div class="text-sm text-base-content/60">
                  {organisations().length} total
                </div>
              </div>

              <div class="overflow-x-auto">
                <table class="table table-sm">
                  <thead>
                    <tr>
                      <th>Name</th>
                      <th>Domain</th>
                    </tr>
                  </thead>
                  <tbody>
                    <Show
                      when={organisations().length > 0}
                      fallback={
                        <tr>
                          <td colSpan={2} class="text-center py-8 text-base-content/60">
                            No organisations found
                          </td>
                        </tr>
                      }
                    >
                      <For each={organisations()}>
                        {(org) => (
                          <tr class="hover">
                            <td class="font-medium text-sm">{org.name}</td>
                            <td class="text-xs text-base-content/60">{org.domain || "-"}</td>
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
