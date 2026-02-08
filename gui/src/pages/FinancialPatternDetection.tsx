import { A } from "@solidjs/router";
import { createSignal, onMount, Show, For } from "solid-js";
import { HiOutlineArrowPath, HiOutlineSparkles } from "solid-icons/hi";
import type { FinancialEmailScanResponse } from "../api-types/types";
import { getApiUrl } from "../config/api";
import FinancialPageLayout from "../components/FinancialPageLayout";

const emptyResponse: FinancialEmailScanResponse = {
  total_emails_scanned: 0,
  total_matched_emails: 0,
  senders: [],
};

export default function FinancialPatternDetection() {
  const [scanResult, setScanResult] =
    createSignal<FinancialEmailScanResponse>(emptyResponse);
  const [loading, setLoading] = createSignal(true);
  const [error, setError] = createSignal<string | null>(null);

  const fetchScan = async () => {
    setLoading(true);
    setError(null);

    try {
      const response = await fetch(getApiUrl("/api/financial/email-scan"), {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({}),
      });

      if (!response.ok) {
        throw new Error(`Failed to run scan: ${response.status}`);
      }

      const data: FinancialEmailScanResponse = await response.json();
      setScanResult(data);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to run scan");
    } finally {
      setLoading(false);
    }
  };

  onMount(() => {
    fetchScan();
  });

  const footerActions = (
    <>
      <A href="/financial/patterns" class="btn btn-ghost btn-sm">
        Back to Patterns
      </A>
      <button class="btn btn-primary btn-sm" onClick={fetchScan}>
        <HiOutlineArrowPath class="w-4 h-4" />
        Refresh
      </button>
    </>
  );

  return (
    <FinancialPageLayout
      title="Financial Overview: Detect Patterns"
      subtitle="Detect Patterns With AI"
      footer={footerActions}
    >
      <div class="space-y-6">
        <Show when={error()}>
          <div class="alert alert-error">
            <span>{error()}</span>
          </div>
        </Show>

        <Show when={loading()}>
          <div class="flex items-center gap-2 text-sm text-base-content/70">
            <span class="loading loading-spinner loading-sm"></span>
            Running scan...
          </div>
        </Show>

        <Show when={!loading()}>
          <div class="stats stats-vertical lg:stats-horizontal shadow bg-base-100">
            <div class="stat">
              <div class="stat-figure text-primary">
                <HiOutlineSparkles class="w-8 h-8" />
              </div>
              <div class="stat-title">Emails Scanned</div>
              <div class="stat-value">
                {scanResult().total_emails_scanned}
              </div>
            </div>
            <div class="stat">
              <div class="stat-title">Matched Emails</div>
              <div class="stat-value text-secondary">
                {scanResult().total_matched_emails}
              </div>
              <div class="stat-desc">
                Senders matched: {scanResult().senders.length}
              </div>
            </div>
          </div>

          <div class="card bg-base-100 shadow">
            <div class="card-body">
              <div class="flex items-center justify-between">
                <h2 class="text-lg font-semibold">Senders by volume</h2>
                <span class="text-xs text-base-content/60">
                  {scanResult().senders.length} senders
                </span>
              </div>
              <div class="overflow-x-auto">
                <table class="table table-zebra">
                  <thead>
                    <tr>
                      <th>Sender</th>
                      <th class="text-right">Matched Emails</th>
                    </tr>
                  </thead>
                  <tbody>
                    <For each={scanResult().senders}>
                      {(sender) => (
                        <tr>
                          <td class="font-mono text-xs">
                            {sender.sender_email}
                          </td>
                          <td class="text-right">{sender.matched_count}</td>
                        </tr>
                      )}
                    </For>
                  </tbody>
                </table>
              </div>
              <Show when={scanResult().senders.length === 0}>
                <div class="text-sm text-base-content/60">
                  No matching senders yet.
                </div>
              </Show>
            </div>
          </div>
        </Show>
      </div>
    </FinancialPageLayout>
  );
}
