import { A } from "@solidjs/router";
import { createEffect, createSignal, For, Show } from "solid-js";
import type { Bill, BillStatus, ListFinancialBillsResponse } from "../api-types/types";
import { getApiUrl } from "../config/api";
import FinancialPageLayout from "../components/FinancialPageLayout";

export default function FinancialBills() {
  const [loading, setLoading] = createSignal(true);
  const [error, setError] = createSignal<string | null>(null);
  const [bills, setBills] = createSignal<Bill[]>([]);

  const [startDate, setStartDate] = createSignal("");
  const [endDate, setEndDate] = createSignal("");
  const [currentPage, setCurrentPage] = createSignal(1);
  const [totalPages, setTotalPages] = createSignal(1);
  const [totalCount, setTotalCount] = createSignal(0);

  const getStatusBadgeClass = (status: BillStatus) => {
    switch (status) {
      case "paid":
        return "badge-success";
      case "overdue":
        return "badge-error";
      case "unpaid":
        return "badge-warning";
      case "received":
        return "badge-info";
      case "cancelled":
      default:
        return "badge-ghost";
    }
  };

  const formatDate = (timestamp: bigint | null) => {
    if (!timestamp) return "-";
    return new Date(Number(timestamp)).toLocaleDateString();
  };

  const fetchBills = async (page = 1) => {
    setLoading(true);
    setError(null);

    try {
      const params = new URLSearchParams();
      params.set("page", page.toString());
      if (startDate()) params.set("start_due_date", startDate());
      if (endDate()) params.set("end_due_date", endDate());

      const response = await fetch(
        getApiUrl(`/api/financial/bills?${params.toString()}`),
      );

      if (!response.ok) {
        throw new Error(`Failed to fetch bills: ${response.status}`);
      }

      const data: ListFinancialBillsResponse = await response.json();

      setBills(data.bills || []);
      setCurrentPage(data.pagination.page);
      setTotalPages(data.pagination.total_pages);
      setTotalCount(data.pagination.total_count);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to fetch bills");
      console.error("Error fetching bills:", err);
    } finally {
      setLoading(false);
    }
  };

  const applyFilters = () => {
    setCurrentPage(1);
    fetchBills(1);
  };

  const clearFilters = () => {
    setStartDate("");
    setEndDate("");
    setCurrentPage(1);
    fetchBills(1);
  };

  createEffect(() => {
    fetchBills();
  });

  const footerActions = null;

  return (
    <FinancialPageLayout
      title="Bills"
      subtitle="Track bills and due dates extracted from your documents"
      footer={footerActions}
    >
      <div class="space-y-6">
        <Show when={error()}>
          <div class="alert alert-error">
            <div>
              <h3 class="font-bold">Error loading bills</h3>
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
          <div>
            <div class="bg-base-100 rounded-lg p-4 mb-4 shadow-sm border border-base-300">
              <div class="flex flex-wrap gap-3 items-end">
                <div class="form-control flex-1 min-w-[140px]">
                  <label class="label">
                    <span class="label-text text-xs font-medium">Start Due Date</span>
                  </label>
                  <input
                    type="date"
                    class="input input-sm input-bordered w-full"
                    value={startDate()}
                    onInput={(e) => setStartDate(e.currentTarget.value)}
                  />
                </div>

                <div class="form-control flex-1 min-w-[140px]">
                  <label class="label">
                    <span class="label-text text-xs font-medium">End Due Date</span>
                  </label>
                  <input
                    type="date"
                    class="input input-sm input-bordered w-full"
                    value={endDate()}
                    onInput={(e) => setEndDate(e.currentTarget.value)}
                  />
                </div>

                <div class="flex gap-2">
                  <button
                    class="btn btn-sm btn-primary"
                    onClick={applyFilters}
                    disabled={loading()}
                  >
                    Apply
                  </button>
                  <button
                    class="btn btn-sm btn-ghost"
                    onClick={clearFilters}
                    disabled={loading() || (!startDate() && !endDate())}
                  >
                    Clear
                  </button>
                </div>
              </div>
            </div>

            <div class="card bg-base-100 shadow-sm border border-base-300">
              <div class="card-body">
                <div class="flex items-center justify-between mb-4">
                  <h2 class="card-title">Bills</h2>
                  <div class="text-sm text-base-content/60">
                    {totalCount()} total bills
                  </div>
                </div>

                <div class="overflow-x-auto">
                  <table class="table table-sm">
                    <thead>
                      <tr>
                        <th>Due Date</th>
                        <th>Bill</th>
                        <th>Amount</th>
                        <th>Currency</th>
                        <th>Status</th>
                      </tr>
                    </thead>
                    <tbody>
                      <Show
                        when={bills().length > 0}
                        fallback={
                          <tr>
                            <td colSpan={5} class="text-center py-8 text-base-content/60">
                              No bills found
                            </td>
                          </tr>
                        }
                      >
                        <For each={bills()}>
                          {(bill) => (
                            <tr class="hover">
                              <td class="text-xs">{formatDate(bill.due_date)}</td>
                              <td>
                                <div class="font-medium text-sm truncate max-w-[320px]" title={bill.document_reference || "Unknown Reference"}>
                                  {bill.document_reference || "Unknown Reference"}
                                </div>
                              </td>
                              <td>{bill.total_amount ?? "-"}</td>
                              <td class="text-xs uppercase">{bill.currency ?? "-"}</td>
                              <td>
                                <span
                                  class={`badge badge-sm ${getStatusBadgeClass(bill.status)}`}
                                >
                                  {bill.status}
                                </span>
                              </td>
                            </tr>
                          )}
                        </For>
                      </Show>
                    </tbody>
                  </table>
                </div>

                <Show when={totalPages() > 1}>
                  <div class="flex items-center justify-between mt-4 pt-4 border-t border-base-300">
                    <div class="text-sm text-base-content/60">
                      Page {currentPage()} of {totalPages()}
                    </div>
                    <div class="join">
                      <button
                        class="join-item btn btn-sm"
                        disabled={currentPage() === 1 || loading()}
                        onClick={() => fetchBills(currentPage() - 1)}
                      >
                        «
                      </button>
                      <button class="join-item btn btn-sm btn-disabled">
                        Page {currentPage()}
                      </button>
                      <button
                        class="join-item btn btn-sm"
                        disabled={currentPage() === totalPages() || loading()}
                        onClick={() => fetchBills(currentPage() + 1)}
                      >
                        »
                      </button>
                    </div>
                  </div>
                </Show>
              </div>
            </div>
          </div>
        </Show>
      </div>
    </FinancialPageLayout>
  );
}
