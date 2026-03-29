import { createEffect, createSignal, For, Show } from "solid-js";
import type { Order, OrdersResponse } from "../api-types/types";
import { getApiUrl } from "../config/api";

export default function Orders() {
  const [loading, setLoading] = createSignal(true);
  const [error, setError] = createSignal<string | null>(null);
  const [orders, setOrders] = createSignal<Order[]>([]);

  const fetchOrders = async () => {
    setLoading(true);
    setError(null);

    try {
      const response = await fetch(getApiUrl("/api/orders"));

      if (!response.ok) {
        throw new Error(`Failed to fetch orders: ${response.status}`);
      }

      const data: OrdersResponse = await response.json();
      setOrders(data.orders || []);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to fetch orders");
      console.error("Error fetching orders:", err);
    } finally {
      setLoading(false);
    }
  };

  const formatDate = (timestamp: bigint | null) => {
    if (!timestamp) return "-";
    return new Date(Number(timestamp)).toLocaleDateString();
  };

  createEffect(() => {
    fetchOrders();
  });

  return (
    <div class="h-full min-h-0 flex flex-col overflow-hidden">
      <header class="pt-4 px-4 md:pt-8 md:px-8 mb-6">
        <h1 class="text-3xl font-bold mb-2">Orders</h1>
        <p class="text-base-content/60">
          E-commerce orders extracted from your emails
        </p>
      </header>

      <main class="flex-1 min-h-0 overflow-y-auto px-4 md:px-8">
        <Show when={error()}>
          <div class="alert alert-error">
            <div>
              <h3 class="font-bold">Error loading orders</h3>
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
                <h2 class="card-title">Orders</h2>
                <div class="text-sm text-base-content/60">
                  {orders().length} total
                </div>
              </div>

              <div class="overflow-x-auto">
                <table class="table table-sm">
                  <thead>
                    <tr>
                      <th>Reference</th>
                      <th>Date</th>
                      <th>Status</th>
                      <th>Amount</th>
                    </tr>
                  </thead>
                  <tbody>
                    <Show
                      when={orders().length > 0}
                      fallback={
                        <tr>
                          <td colSpan={4} class="text-center py-8 text-base-content/60">
                            No orders found
                          </td>
                        </tr>
                      }
                    >
                      <For each={orders()}>
                        {(order) => (
                          <tr class="hover">
                            <td class="font-medium text-sm truncate max-w-[200px]">
                              {order.order_reference || "-"}
                            </td>
                            <td class="text-xs">{formatDate(order.order_date)}</td>
                            <td>
                              <span class={`badge badge-sm ${order.status === "delivered" ? "badge-success" : order.status === "cancelled" ? "badge-error" : "badge-ghost"}`}>
                                {order.status || "-"}
                              </span>
                            </td>
                            <td>
                              {order.total_amount != null ? `${order.total_amount} ${order.currency || ""}` : "-"}
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
