import { A } from "@solidjs/router";
import { createSignal, onMount, Show, For } from "solid-js";
import {
  HiOutlineArrowPath,
  HiOutlineCheckCircle,
  HiOutlineXCircle,
} from "solid-icons/hi";
import type { FinancialPattern } from "../api-types/types";
import { getApiUrl } from "../config/api";
import FinancialPageLayout from "../components/FinancialPageLayout";
import { usePrivacyMode } from "../contexts/PrivacyMode";

type PatternsResponse = {
  patterns: FinancialPattern[];
  total: number;
};

export default function FinancialPatterns() {
  const { isEnabled } = usePrivacyMode();
  const [patterns, setPatterns] = createSignal<FinancialPattern[]>([]);
  const [loading, setLoading] = createSignal(true);
  const [error, setError] = createSignal<string | null>(null);
  const [activeOnly, setActiveOnly] = createSignal(false);

  const fetchPatterns = async () => {
    setLoading(true);
    setError(null);

    try {
      const url = getApiUrl(
        `/api/financial/patterns?active_only=${activeOnly() ? "true" : "false"}`,
      );
      const response = await fetch(url);
      if (!response.ok) {
        throw new Error(`Failed to fetch patterns: ${response.status}`);
      }
      const data: PatternsResponse = await response.json();
      setPatterns(data.patterns || []);
    } catch (err) {
      setError(
        err instanceof Error ? err.message : "Failed to load patterns",
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
    fetchPatterns();
  });

  const footerActions = (
    <>
      <A href="/financial" class="btn btn-ghost btn-sm">
        Back to Financial
      </A>
      <A
        href="/financial/patterns/detect"
        class="btn btn-secondary btn-sm"
      >
        Detect Patterns With AI
      </A>
      <button class="btn btn-primary btn-sm" onClick={fetchPatterns}>
        <HiOutlineArrowPath class="w-4 h-4" />
        Refresh
      </button>
    </>
  );

  return (
    <FinancialPageLayout
      title="Financial Overview: Patterns"
      subtitle="Regex patterns used by the extractor to classify and extract transactions."
      footer={footerActions}
    >
      <div class="space-y-6">
        <div class="flex flex-wrap items-center gap-3">
          <label class="label cursor-pointer gap-2">
            <input
              type="checkbox"
              class="checkbox checkbox-sm"
              checked={activeOnly()}
              onChange={(event) => {
                setActiveOnly(event.currentTarget.checked);
                fetchPatterns();
              }}
            />
            <span class="label-text text-sm">Active only</span>
          </label>
          <span class="text-xs text-base-content/60">
            {patterns().length} patterns
          </span>
        </div>

        <Show when={error()}>
          <div class="alert alert-error">
            <span>{error()}</span>
          </div>
        </Show>

        <Show when={loading()}>
          <div class="flex items-center gap-2 text-sm text-base-content/70">
            <span class="loading loading-spinner loading-sm"></span>
            Loading patterns...
          </div>
        </Show>

        <Show when={!loading()}>
          <div class="card bg-base-100 shadow">
            <div class="card-body">
              <div class="overflow-x-auto">
                <table class="table table-zebra">
                  <thead>
                    <tr>
                      <th>Name</th>
                      <th>Sender</th>
                      <th>Document</th>
                      <th>Status</th>
                      <th>Confidence</th>
                      <th>Groups</th>
                      <th>Active</th>
                      <th>Matches</th>
                      <th>Last Matched</th>
                      <th>Regex</th>
                    </tr>
                  </thead>
                  <tbody>
                    <For each={patterns()}>
                      {(pattern) => (
                        <tr>
                          <td>
                            <div
                              class="font-medium"
                              classList={{ "privacy-blur": isEnabled() }}
                            >
                              {pattern.name}
                            </div>
                            <Show when={pattern.description}>
                              <div class="text-xs text-base-content/60">
                                {pattern.description}
                              </div>
                            </Show>
                          </td>
                          <td class="text-xs">
                            <span
                              class="font-mono"
                              classList={{ "privacy-blur": isEnabled() }}
                            >
                              {pattern.sender_email ?? "—"}
                            </span>
                          </td>
                          <td>
                            <span
                              class="badge badge-outline"
                              classList={{ "privacy-blur": isEnabled() }}
                            >
                              {pattern.document_type}
                            </span>
                          </td>
                          <td>
                            <span class="badge badge-ghost">
                              {pattern.status}
                            </span>
                          </td>
                          <td>{pattern.confidence.toFixed(2)}</td>
                          <td class="text-xs">
                            <div>Amount: {pattern.amount_group}</div>
                            <div>
                              Vendor: {pattern.vendor_group ?? "—"}
                            </div>
                            <div>Date: {pattern.date_group ?? "—"}</div>
                          </td>
                          <td>
                            <span
                              class={`badge ${pattern.is_active ? "badge-success" : "badge-ghost"}`}
                            >
                              {pattern.is_active ? (
                                <HiOutlineCheckCircle class="w-4 h-4" />
                              ) : (
                                <HiOutlineXCircle class="w-4 h-4" />
                              )}
                              <span class="ml-1">
                                {pattern.is_active ? "Active" : "Inactive"}
                              </span>
                            </span>
                            <Show when={pattern.is_default}>
                              <div class="text-xs text-base-content/60 mt-1">
                                Default
                              </div>
                            </Show>
                          </td>
                          <td classList={{ "privacy-blur": isEnabled() }}>
                            {pattern.match_count}
                          </td>
                          <td>{formatTimestamp(pattern.last_matched_at)}</td>
                          <td class="font-mono text-xs whitespace-pre-wrap break-all max-w-xl">
                            {pattern.regex_pattern}
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
      </div>
    </FinancialPageLayout>
  );
}
