import { createEffect, createSignal, For, Show } from "solid-js";
import { A } from "@solidjs/router";
import type { OrganisationWithCounts, OrganisationsWithCountsResponse } from "../api-types/types";
import { getApiUrl } from "../config/api";

export default function Organisations() {
  const [loading, setLoading] = createSignal(true);
  const [error, setError] = createSignal<string | null>(null);
  const [organisations, setOrganisations] = createSignal<OrganisationWithCounts[]>([]);

  const [isExtracting, setIsExtracting] = createSignal(false);
  const [extractError, setExtractError] = createSignal("");
  const [extractStats, setExtractStats] = createSignal<string | null>(null);

  const fetchOrganisations = async () => {
    setLoading(true);
    setError(null);

    try {
      const response = await fetch(getApiUrl("/api/organisations"));

      if (!response.ok) {
        throw new Error(`Failed to fetch organisations: ${response.status}`);
      }

      const data: OrganisationsWithCountsResponse = await response.json();
      setOrganisations(data.organisations || []);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to fetch organisations");
      console.error("Error fetching organisations:", err);
    } finally {
      setLoading(false);
    }
  };

  const extractContacts = async () => {
    setIsExtracting(true);
    setExtractError("");
    setExtractStats(null);

    try {
      const response = await fetch(getApiUrl("/api/contact-extractor/run"), {
        method: "POST",
      });

      if (!response.ok) {
        throw new Error(`Extraction failed: ${response.statusText}`);
      }

      const stats = await response.json();
      setExtractStats(
        `Done — ${stats.organisations_created} new organisations added, ${stats.organisations_skipped} already known.`
      );
      await fetchOrganisations();
    } catch (err) {
      setExtractError(
        err instanceof Error ? err.message : "Extraction failed"
      );
    } finally {
      setIsExtracting(false);
    }
  };

  createEffect(() => {
    fetchOrganisations();
  });

  return (
    <div class="h-full min-h-0 flex flex-col overflow-hidden">
      <header class="pt-4 px-4 md:pt-8 md:px-8 mb-6">
        <div class="flex items-start justify-between gap-4">
          <div>
            <h1 class="text-3xl font-bold mb-2">Organisations</h1>
            <p class="text-base-content/60">
              Companies and businesses from your emails
            </p>
          </div>
          <button
            class="btn btn-primary shrink-0"
            onClick={extractContacts}
            disabled={isExtracting()}
          >
            {isExtracting() ? (
              <>
                <span class="loading loading-spinner loading-sm"></span>
                Extracting…
              </>
            ) : (
              "Extract People / Organisations"
            )}
          </button>
        </div>

        <Show when={extractStats()}>
          <div class="alert alert-success mt-3 py-2 text-sm">{extractStats()}</div>
        </Show>
        <Show when={extractError()}>
          <div class="alert alert-error mt-3 py-2 text-sm">{extractError()}</div>
        </Show>
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
                      <th>Email</th>
                      <th class="text-right">Received</th>
                      <th class="text-right">Sent to</th>
                    </tr>
                  </thead>
                  <tbody>
                    <Show
                      when={organisations().length > 0}
                      fallback={
                        <tr>
                          <td colSpan={4} class="text-center py-8 text-base-content/60">
                            No organisations found
                          </td>
                        </tr>
                      }
                    >
                      <For each={organisations()}>
                        {(row) => (
                          <tr class="hover">
                            <td class="font-medium text-sm">{row.name}</td>
                            <td class="text-xs text-base-content/60">{row.email || "-"}</td>
                            <td class="text-right">
                              <Show when={row.email && row.received_count > 0} fallback={
                                <span class="text-xs text-base-content/40">{row.received_count}</span>
                              }>
                                <A
                                  href={`/emails?q=${encodeURIComponent(`from:${row.email}`)}`}
                                  class="link link-primary text-xs"
                                >
                                  {row.received_count}
                                </A>
                              </Show>
                            </td>
                            <td class="text-right">
                              <Show when={row.email && row.in_to_count > 0} fallback={
                                <span class="text-xs text-base-content/40">{row.in_to_count}</span>
                              }>
                                <A
                                  href={`/emails?q=${encodeURIComponent(`to:${row.email}`)}`}
                                  class="link link-primary text-xs"
                                >
                                  {row.in_to_count}
                                </A>
                              </Show>
                            </td>
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
