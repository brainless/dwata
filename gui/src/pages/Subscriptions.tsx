import { createEffect, createSignal, For, Show } from "solid-js";
import type { Subscription, SubscriptionsResponse } from "../api-types/types";
import { getApiUrl } from "../config/api";

export default function Subscriptions() {
  const [loading, setLoading] = createSignal(true);
  const [error, setError] = createSignal<string | null>(null);
  const [subscriptions, setSubscriptions] = createSignal<Subscription[]>([]);

  const fetchSubscriptions = async () => {
    setLoading(true);
    setError(null);

    try {
      const response = await fetch(getApiUrl("/api/subscriptions"));

      if (!response.ok) {
        throw new Error(`Failed to fetch subscriptions: ${response.status}`);
      }

      const data: SubscriptionsResponse = await response.json();
      setSubscriptions(data.subscriptions || []);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to fetch subscriptions");
      console.error("Error fetching subscriptions:", err);
    } finally {
      setLoading(false);
    }
  };

  const formatDate = (timestamp: bigint | null) => {
    if (!timestamp) return "-";
    return new Date(Number(timestamp)).toLocaleDateString();
  };

  createEffect(() => {
    fetchSubscriptions();
  });

  return (
    <div class="h-full min-h-0 flex flex-col overflow-hidden">
      <header class="pt-4 px-4 md:pt-8 md:px-8 mb-6">
        <h1 class="text-3xl font-bold mb-2">Subscriptions</h1>
        <p class="text-base-content/60">
          Recurring subscriptions extracted from your emails
        </p>
      </header>

      <main class="flex-1 min-h-0 overflow-y-auto px-4 md:px-8">
        <Show when={error()}>
          <div class="alert alert-error">
            <div>
              <h3 class="font-bold">Error loading subscriptions</h3>
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
                <h2 class="card-title">Subscriptions</h2>
                <div class="text-sm text-base-content/60">
                  {subscriptions().length} total
                </div>
              </div>

              <div class="overflow-x-auto">
                <table class="table table-sm">
                  <thead>
                    <tr>
                      <th>Service</th>
                      <th>Plan</th>
                      <th>Billing</th>
                      <th>Amount</th>
                      <th>Next Billing</th>
                    </tr>
                  </thead>
                  <tbody>
                    <Show
                      when={subscriptions().length > 0}
                      fallback={
                        <tr>
                          <td colSpan={5} class="text-center py-8 text-base-content/60">
                            No subscriptions found
                          </td>
                        </tr>
                      }
                    >
                      <For each={subscriptions()}>
                        {(sub) => (
                          <tr class="hover">
                            <td class="font-medium text-sm">{sub.service_name}</td>
                            <td class="text-xs">{sub.plan_name || "-"}</td>
                            <td class="text-xs capitalize">{sub.billing_cycle?.replace("_", " ") || "-"}</td>
                            <td>{sub.amount != null ? `${sub.amount} ${sub.currency || ""}` : "-"}</td>
                            <td class="text-xs">{formatDate(sub.next_billing_date)}</td>
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
