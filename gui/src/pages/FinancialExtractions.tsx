import { A } from "@solidjs/router";
import { createSignal, onMount, Show, For } from "solid-js";
import {
  HiOutlineArrowPath,
  HiOutlineClock,
  HiOutlineDocumentText,
  HiOutlineEnvelope,
} from "solid-icons/hi";
import type {
  FinancialExtractionAttempt,
  FinancialExtractionSummary,
} from "../api-types/types";
import { getApiUrl } from "../config/api";

export default function FinancialExtractions() {
  const [summary, setSummary] = createSignal<FinancialExtractionSummary | null>(
    null,
  );
  const [attempts, setAttempts] = createSignal<FinancialExtractionAttempt[]>(
    [],
  );
  const [loading, setLoading] = createSignal(true);
  const [error, setError] = createSignal<string | null>(null);

  const fetchSummary = async () => {
    setLoading(true);
    setError(null);

    try {
      const [summaryResponse, attemptsResponse] = await Promise.all([
        fetch(getApiUrl("/api/financial/extractions/summary")),
        fetch(getApiUrl("/api/financial/extractions/attempts")),
      ]);

      if (!summaryResponse.ok) {
        throw new Error(`Failed to fetch summary: ${summaryResponse.status}`);
      }
      if (!attemptsResponse.ok) {
        throw new Error(`Failed to fetch attempts: ${attemptsResponse.status}`);
      }

      const summaryData: FinancialExtractionSummary =
        await summaryResponse.json();
      const attemptsData = await attemptsResponse.json();

      setSummary(summaryData);
      setAttempts(attemptsData.attempts || []);
    } catch (err) {
      setError(
        err instanceof Error ? err.message : "Failed to load extraction status",
      );
    } finally {
      setLoading(false);
    }
  };

  const formatTimestamp = (timestamp?: number | null) => {
    if (!timestamp) {
      return "Not yet";
    }
    return new Date(timestamp * 1000).toLocaleString();
  };

  onMount(() => {
    fetchSummary();
  });

  return (
    <div class="p-6 space-y-6">
      <div class="flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between">
        <div>
          <h1 class="text-3xl font-bold">Financial Extractions</h1>
          <p class="text-sm text-base-content/70">
            High-level status for processed sources in financial extraction.
          </p>
        </div>
        <div class="flex gap-2">
          <A href="/financial" class="btn btn-ghost btn-sm">
            Back to Financial
          </A>
          <button
            class="btn btn-primary btn-sm"
            onClick={() => fetchSummary()}
          >
            <HiOutlineArrowPath class="w-4 h-4" />
            Refresh
          </button>
        </div>
      </div>

      <Show when={error()}>
        <div class="alert alert-error">
          <HiOutlineDocumentText class="w-5 h-5" />
          <span>{error()}</span>
        </div>
      </Show>

      <Show when={loading()}>
        <div class="flex items-center gap-2 text-sm text-base-content/70">
          <span class="loading loading-spinner loading-sm"></span>
          Loading extraction summary...
        </div>
      </Show>

      <Show when={!loading() && summary()}>
        <div class="stats stats-vertical lg:stats-horizontal shadow bg-base-100">
          <div class="stat">
            <div class="stat-figure text-primary">
              <HiOutlineEnvelope class="w-8 h-8" />
            </div>
            <div class="stat-title">Sources Processed</div>
            <div class="stat-value">{summary()!.source_count}</div>
            <div class="stat-desc">
              Unique sources with extracted transactions
            </div>
          </div>

          <div class="stat">
            <div class="stat-figure text-secondary">
              <HiOutlineDocumentText class="w-8 h-8" />
            </div>
            <div class="stat-title">Transactions Captured</div>
            <div class="stat-value">{summary()!.transaction_count}</div>
            <div class="stat-desc">Total transactions from those sources</div>
          </div>

          <div class="stat">
            <div class="stat-figure text-accent">
              <HiOutlineClock class="w-8 h-8" />
            </div>
            <div class="stat-title">Last Extracted</div>
            <div class="stat-value text-lg">
              {formatTimestamp(summary()!.last_extracted_at)}
            </div>
            <div class="stat-desc">
              <span
                class={`badge badge-outline ${summary()!.source_count > 0 ? "badge-success" : "badge-ghost"}`}
              >
                {summary()!.source_count > 0 ? "Active" : "No extractions yet"}
              </span>
            </div>
          </div>
        </div>
      </Show>

      <Show when={!loading() && attempts().length > 0}>
        <div class="card bg-base-100 shadow">
          <div class="card-body">
            <div class="flex items-center justify-between">
              <h2 class="card-title">Extraction Attempts</h2>
              <span class="text-xs text-base-content/60">
                Latest {attempts().length} attempts
              </span>
            </div>
            <div class="overflow-x-auto">
              <table class="table table-zebra">
                <thead>
                  <tr>
                    <th>Source</th>
                    <th>Account ID</th>
                    <th>Attempted At</th>
                    <th>Items Scanned</th>
                    <th>Transactions</th>
                    <th>Status</th>
                  </tr>
                </thead>
                <tbody>
                  <For each={attempts()}>
                    {(attempt) => (
                      <tr>
                        <td class="uppercase text-xs tracking-wide">
                          {attempt.source_type}
                        </td>
                        <td>{attempt.source_account_id}</td>
                        <td>{formatTimestamp(attempt.attempted_at)}</td>
                        <td>{attempt.total_items_scanned}</td>
                        <td>{attempt.transactions_extracted}</td>
                        <td>
                          <div class="flex flex-col gap-1">
                            <span
                              class={`badge badge-outline ${attempt.status === "completed" ? "badge-success" : attempt.status === "failed" ? "badge-error" : "badge-ghost"}`}
                            >
                              {attempt.status}
                            </span>
                            <Show when={attempt.error_message}>
                              <span class="text-xs text-error">
                                {attempt.error_message}
                              </span>
                            </Show>
                          </div>
                        </td>
                      </tr>
                    )}
                  </For>
                </tbody>
              </table>
            </div>
          </div>
        </div>
      </Show>

      <Show when={!loading() && summary() && summary()!.source_count === 0}>
        <div class="card bg-base-200">
          <div class="card-body">
            <h2 class="card-title">No sources extracted yet</h2>
            <p class="text-sm text-base-content/70">
              Trigger a financial extraction to start capturing transactions from
              your email sources.
            </p>
            <div class="card-actions">
              <A href="/financial" class="btn btn-primary btn-sm">
                Run Extraction
              </A>
            </div>
          </div>
        </div>
      </Show>
    </div>
  );
}
