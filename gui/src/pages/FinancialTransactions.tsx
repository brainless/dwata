import {
  HiOutlineArrowTrendingUp,
  HiOutlineArrowTrendingDown,
  HiOutlineCurrencyDollar,
  HiOutlineExclamationTriangle,
  HiOutlineDocumentText,
  HiOutlineArrowPath,
  HiOutlineArrowUpTray,
} from "solid-icons/hi";
import { createSignal, createEffect, For, Show } from "solid-js";
import type {
  FinancialTransaction,
  TransactionCategory,
  TransactionStatus,
  CategoryBreakdown,
} from "../api-types/types";
import { getApiUrl } from "../config/api";
import FinancialPageLayout from "../components/FinancialPageLayout";

export default function FinancialTransactions() {
  const [loading, setLoading] = createSignal(true);
  const [error, setError] = createSignal<string | null>(null);

  const [transactions, setTransactions] = createSignal<FinancialTransaction[]>(
    [],
  );
  const [isExtracting, setIsExtracting] = createSignal(false);
  const [credentials, setCredentials] = createSignal<any[]>([]);

  // Filter state
  const [startDate, setStartDate] = createSignal("");
  const [endDate, setEndDate] = createSignal("");
  const [currentPage, setCurrentPage] = createSignal(1);
  const [totalPages, setTotalPages] = createSignal(1);
  const [totalCount, setTotalCount] = createSignal(0);

  const formatCurrency = (amount: number, currency?: string) => {
    const code = (currency || "USD").toUpperCase();
    try {
      return new Intl.NumberFormat(undefined, {
        style: "currency",
        currency: code,
      }).format(amount);
    } catch {
      return `${amount.toLocaleString()} ${code}`;
    }
  };

  const fetchFinancialData = async (page = 1) => {
    setLoading(true);
    setError(null);

    try {
      // Build transactions query params
      const transactionParams = new URLSearchParams();
      transactionParams.set("page", page.toString());
      if (startDate()) {
        transactionParams.set("start_date", startDate());
      }
      if (endDate()) {
        transactionParams.set("end_date", endDate());
      }

      const response = await fetch(
        getApiUrl(`/api/financial/transactions?${transactionParams}`),
      );

      if (!response.ok) {
        throw new Error(
          `Failed to fetch transactions: ${response.status}`,
        );
      }

      const data = await response.json();

      setTransactions(data.transactions || []);

      if (data.pagination) {
        setCurrentPage(data.pagination.page);
        setTotalPages(data.pagination.total_pages);
        setTotalCount(data.pagination.total_count);
      }
    } catch (err) {
      setError(
        err instanceof Error ? err.message : "Failed to fetch financial data",
      );
      console.error("Error fetching financial data:", err);
    } finally {
      setLoading(false);
    }
  };

  const applyFilters = () => {
    setCurrentPage(1);
    fetchFinancialData(1);
  };

  const clearFilters = () => {
    setStartDate("");
    setEndDate("");
    setCurrentPage(1);
    fetchFinancialData(1);
  };

  const fetchCredentials = async () => {
    try {
      const response = await fetch(getApiUrl("/api/credentials"));
      const data = await response.json();
      setCredentials(data.credentials || []);
    } catch (error) {
      console.error("Failed to fetch credentials:", error);
    }
  };

  const triggerExtraction = async () => {
    setIsExtracting(true);
    try {
      const emailCredentials = credentials().filter((cred: any) => {
        const isImapType = cred.credential_type === "imap";
        const isImapOauth =
          cred.credential_type === "oauth" &&
          cred.service_name?.includes("imap");
        return isImapType || isImapOauth;
      });

      for (const cred of emailCredentials) {
        const response = await fetch(getApiUrl("/api/financial/extract"), {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({ credential_id: Number(cred.id) }),
        });

        if (!response.ok) {
          console.error(
            `Extraction failed for credential ${cred.id}:`,
            response.status,
          );
        }
      }

      await fetchFinancialData();
    } catch (err) {
      console.error("Error triggering extraction:", err);
    } finally {
      setIsExtracting(false);
    }
  };

  createEffect(() => {
    fetchFinancialData();
    fetchCredentials();
  });

  const categoryBreakdown = (): CategoryBreakdown[] => {
    const all = transactions();
    const totals = new Map<
      TransactionCategory,
      { amount: number; count: number }
    >();

    for (const txn of all) {
      if (txn.category) {
        const current = totals.get(txn.category) || { amount: 0, count: 0 };
        totals.set(txn.category, {
          amount: current.amount + Math.abs(txn.amount),
          count: current.count + 1,
        });
      }
    }

    const totalAmount = Array.from(totals.values()).reduce(
      (sum, v) => sum + v.amount,
      0,
    );

    return Array.from(totals.entries())
      .map(([category, data]) => ({
        category,
        amount: data.amount,
        percentage:
          totalAmount > 0 ? Math.round((data.amount / totalAmount) * 100) : 0,
        transaction_count: data.count,
      }))
      .sort((a, b) => b.amount - a.amount);
  };

  const getStatusBadgeClass = (status: TransactionStatus) => {
    switch (status) {
      case "paid":
        return "badge-success";
      case "cancelled":
        return "badge-ghost";
      case "refunded":
        return "badge-info";
      default:
        return "badge-ghost";
    }
  };

  const getCategoryIcon = (category: TransactionCategory | undefined) => {
    switch (category) {
      case "income":
        return HiOutlineArrowTrendingUp;
      case "expense":
        return HiOutlineArrowTrendingDown;
      default:
        return HiOutlineCurrencyDollar;
    }
  };

  const footerActions = (
    <>
      <button
        class="btn btn-outline btn-sm gap-2"
        disabled={loading() || isExtracting()}
        onClick={triggerExtraction}
      >
        {isExtracting() ? (
          <HiOutlineArrowPath class="w-5 h-5 animate-spin" />
        ) : (
          <HiOutlineArrowPath class="w-5 h-5" />
        )}
        {isExtracting() ? "Extracting..." : "Run Extraction"}
      </button>
    </>
  );

  return (
    <FinancialPageLayout
      title="Financial Transactions"
      subtitle="Track income, expenses, and bills from your documents"
      footer={footerActions}
    >
      <div class="space-y-6">

        {/* Error State */}
        <Show when={error()}>
          <div class="alert alert-error">
            <HiOutlineExclamationTriangle class="w-5 h-5" />
            <div>
              <h3 class="font-bold">Error loading financial data</h3>
              <div class="text-sm">{error()}</div>
            </div>
          </div>
        </Show>

        {/* Loading State */}
        <Show when={loading() && !error()}>
          <div class="flex items-center justify-center py-16">
            <span class="loading loading-spinner loading-lg"></span>
          </div>
        </Show>

        {/* Content */}
        <Show when={!loading() && !error()}>
          <div>
          <div class="grid grid-cols-1 gap-6 mb-8">
            {/* Recent Transactions */}
            <div>
              {/* Filter Bar */}
              <div class="bg-base-100 rounded-lg p-4 mb-4 shadow-sm border border-base-300">
                <div class="flex flex-wrap gap-3 items-end">
                  {/* Date Range */}
                  <div class="form-control flex-1 min-w-[140px]">
                    <label class="label">
                      <span class="label-text text-xs font-medium">Start Date</span>
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
                      <span class="label-text text-xs font-medium">End Date</span>
                    </label>
                    <input
                      type="date"
                      class="input input-sm input-bordered w-full"
                      value={endDate()}
                      onInput={(e) => setEndDate(e.currentTarget.value)}
                    />
                  </div>

                  {/* Action Buttons */}
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
                    <h2 class="card-title">Recent Transactions</h2>
                    <div class="text-sm text-base-content/60">
                      {totalCount()} total transactions
                    </div>
                  </div>

                  <Show
                    when={transactions().length > 0}
                    fallback={
                      <div class="text-center py-8 text-base-content/60">
                        No transactions found
                      </div>
                    }
                  >
                    <div class="overflow-x-auto">
                      <table class="table table-sm">
                        <thead>
                          <tr>
                            <th>Date</th>
                            <th>Transaction</th>
                            <th>Amount</th>
                            <th>Currency</th>
                            <th>Status</th>
                          </tr>
                        </thead>
                        <tbody>
                          <For each={transactions()}>
                            {(transaction) => {
                              return (
                                <tr class="hover">
                                  <td class="text-xs">
                                    {new Date(
                                      transaction.transaction_date,
                                    ).toLocaleDateString()}
                                  </td>
                                  <td>
                                    <div class="flex items-center gap-2">
                                      <div>
                                        <div class="font-medium text-sm truncate max-w-[320px]" title={transaction.transaction_reference || "Unknown Reference"}>
                                          {(transaction.transaction_reference || "Unknown Reference").length > 60
                                            ? `${(transaction.transaction_reference || "Unknown Reference").slice(0, 60)}…`
                                            : (transaction.transaction_reference || "Unknown Reference")}
                                        </div>
                                      </div>
                                    </div>
                                  </td>
                                  <td
                                    classList={{
                                      "text-success font-semibold":
                                        transaction.amount > 0,
                                      "text-error": transaction.amount < 0,
                                    }}
                                  >
                                    {transaction.amount}
                                  </td>
                                  <td class="text-xs uppercase">
                                    {transaction.currency}
                                  </td>
                                  <td>
                                    <span
                                      class={`badge badge-sm ${getStatusBadgeClass(transaction.status)}`}
                                    >
                                      {transaction.status}
                                    </span>
                                  </td>
                                </tr>
                              );
                            }}
                          </For>
                        </tbody>
                      </table>
                    </div>

                    {/* Pagination */}
                    <Show when={totalPages() > 1}>
                      <div class="flex items-center justify-between mt-4 pt-4 border-t border-base-300">
                        <div class="text-sm text-base-content/60">
                          Page {currentPage()} of {totalPages()}
                        </div>
                        <div class="join">
                          <button
                            class="join-item btn btn-sm"
                            disabled={currentPage() === 1 || loading()}
                            onClick={() => fetchFinancialData(currentPage() - 1)}
                          >
                            «
                          </button>
                          <button class="join-item btn btn-sm btn-disabled">
                            Page {currentPage()}
                          </button>
                          <button
                            class="join-item btn btn-sm"
                            disabled={currentPage() === totalPages() || loading()}
                            onClick={() => fetchFinancialData(currentPage() + 1)}
                          >
                            »
                          </button>
                        </div>
                      </div>
                    </Show>
                  </Show>
                </div>
              </div>
            </div>

            {/* Upcoming Bills section removed */}
          </div>

          {/* Category Breakdown */}
          <div class="card bg-base-100 shadow-sm border border-base-300">
            <div class="card-body">
              <h2 class="card-title mb-4">Spending by Category</h2>

              <Show
                when={categoryBreakdown().length > 0}
                fallback={
                  <div class="text-center py-8 text-base-content/60">
                    No category data available
                  </div>
                }
              >
                <div class="space-y-4">
                  <For each={categoryBreakdown()}>
                    {(cat) => (
                      <div>
                        <div class="flex justify-between text-sm mb-2">
                          <span class="capitalize font-medium">
                            {cat.category}
                          </span>
                          <span class="text-base-content/60">
                            {formatCurrency(
                              cat.amount,
                              summary()!.currency,
                            )}{" "}
                            ({cat.percentage}%)
                          </span>
                        </div>
                        <div class="flex items-center gap-3">
                          <progress
                            class="progress progress-primary w-full"
                            value={cat.percentage}
                            max="100"
                          ></progress>
                          <span class="text-xs text-base-content/60 min-w-fit">
                            {cat.transaction_count} transactions
                          </span>
                        </div>
                      </div>
                    )}
                  </For>
                </div>
              </Show>
            </div>
          </div>
        </div>

          {/* Empty State (when no data) */}
          <Show when={transactions().length === 0 && !loading()}>
            <div class="flex flex-col items-center justify-center py-16 px-4">
              <div class="text-center max-w-md">
                <HiOutlineDocumentText class="w-16 h-16 mx-auto text-base-content/30 mb-4" />
                <h3 class="text-xl font-semibold mb-2">
                  No financial data yet
                </h3>
                <p class="text-base-content/60 mb-6">
                  Upload your invoices, bills, bank statements, and receipts to
                  get started tracking your financial health.
                </p>
                <button class="btn btn-primary gap-2">
                  <HiOutlineArrowUpTray class="w-5 h-5" />
                  Upload Your First Document
                </button>
              </div>
            </div>
          </Show>
        </Show>
      </div>
    </FinancialPageLayout>
  );
}
