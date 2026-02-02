import {
  HiOutlinePlus,
  HiOutlineArrowTrendingUp,
  HiOutlineArrowTrendingDown,
  HiOutlineCurrencyDollar,
  HiOutlineExclamationTriangle,
  HiOutlineDocumentText,
  HiOutlineCalendar,
  HiOutlineCheckCircle,
  HiOutlineClock,
  HiOutlineArrowUpTray,
  HiOutlineArrowPath,
} from "solid-icons/hi";
import { createSignal, createEffect, For, Show } from "solid-js";
import type {
  FinancialTransaction,
  TransactionCategory,
  TransactionStatus,
  FinancialSummary,
  CategoryBreakdown,
} from "../api-types/types";
import { getApiUrl } from "../config/api";

export default function FinancialHealth() {
  const [selectedPeriod, setSelectedPeriod] = createSignal("month");
  const [loading, setLoading] = createSignal(true);
  const [error, setError] = createSignal<string | null>(null);

  const [summary, setSummary] = createSignal<FinancialSummary | null>(null);
  const [transactions, setTransactions] = createSignal<FinancialTransaction[]>(
    [],
  );
  const [isExtracting, setIsExtracting] = createSignal(false);

  const fetchFinancialData = async () => {
    setLoading(true);
    setError(null);

    try {
      const dates = getPeriodDates(selectedPeriod());

      const [summaryResponse, transactionsResponse] = await Promise.all([
        fetch(
          getApiUrl(
            `/api/financial/summary?start_date=${dates.start}&end_date=${dates.end}`,
          ),
        ),
        fetch(getApiUrl("/api/financial/transactions")),
      ]);

      if (!summaryResponse.ok) {
        throw new Error(`Failed to fetch summary: ${summaryResponse.status}`);
      }

      if (!transactionsResponse.ok) {
        throw new Error(
          `Failed to fetch transactions: ${transactionsResponse.status}`,
        );
      }

      const summaryData: FinancialSummary = await summaryResponse.json();
      const transactionsData = await transactionsResponse.json();

      setSummary(summaryData);
      setTransactions(transactionsData.transactions || []);
    } catch (err) {
      setError(
        err instanceof Error ? err.message : "Failed to fetch financial data",
      );
      console.error("Error fetching financial data:", err);
    } finally {
      setLoading(false);
    }
  };

  const getPeriodDates = (period: string) => {
    const now = new Date();
    const start = new Date();
    let end = new Date();

    switch (period) {
      case "week":
        start.setDate(now.getDate() - 7);
        break;
      case "month":
        start.setDate(1);
        end.setMonth(now.getMonth() + 1);
        end.setDate(0);
        break;
      case "quarter":
        const quarter = Math.floor(now.getMonth() / 3);
        start.setMonth(quarter * 3, 1);
        end.setMonth((quarter + 1) * 3, 0);
        break;
      case "year":
        start.setMonth(0, 1);
        end.setMonth(11, 31);
        break;
    }

    return {
      start: start.toISOString().split("T")[0],
      end: end.toISOString().split("T")[0],
    };
  };

  const triggerExtraction = async () => {
    setIsExtracting(true);
    try {
      const response = await fetch(getApiUrl("/api/financial/extract"), {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({}),
      });

      if (!response.ok) {
        throw new Error(`Extraction failed: ${response.status}`);
      }

      const result = await response.json();
      console.log("Extraction result:", result);

      await fetchFinancialData();
    } catch (err) {
      console.error("Error triggering extraction:", err);
    } finally {
      setIsExtracting(false);
    }
  };

  createEffect(() => {
    fetchFinancialData();
  });

  const upcomingBills = () => {
    const all = transactions();
    return all.filter((t) => t.status === "pending" || t.status === "overdue");
  };

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
      case "pending":
        return "badge-warning";
      case "overdue":
        return "badge-error";
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

  return (
    <div class="p-4 md:p-8">
      {/* Header */}
      <div class="flex flex-col md:flex-row md:items-center md:justify-between mb-8">
        <div>
          <h1 class="text-3xl font-bold mb-2">Financial Health</h1>
          <p class="text-base-content/60">
            Track income, expenses, and bills from your documents
          </p>
        </div>
        <div class="flex gap-2 mt-4 md:mt-0">
          <button
            class="btn btn-outline gap-2"
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
          <button class="btn btn-outline gap-2">
            <HiOutlineArrowUpTray class="w-5 h-5" />
            Upload Documents
          </button>
          <button class="btn btn-primary gap-2">
            <HiOutlinePlus class="w-5 h-5" />
            Add Transaction
          </button>
        </div>
      </div>

      {/* Error State */}
      <Show when={error()}>
        <div class="alert alert-error mb-6">
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
      <Show when={!loading() && !error() && summary()}>
        <div>
          {/* Period Selector */}
          <div class="flex gap-2 mb-6">
            <button
              class="btn btn-sm"
              classList={{ "btn-primary": selectedPeriod() === "week" }}
              onClick={() => setSelectedPeriod("week")}
            >
              Week
            </button>
            <button
              class="btn btn-sm"
              classList={{ "btn-primary": selectedPeriod() === "month" }}
              onClick={() => setSelectedPeriod("month")}
            >
              Month
            </button>
            <button
              class="btn btn-sm"
              classList={{ "btn-primary": selectedPeriod() === "quarter" }}
              onClick={() => setSelectedPeriod("quarter")}
            >
              Quarter
            </button>
            <button
              class="btn btn-sm"
              classList={{ "btn-primary": selectedPeriod() === "year" }}
              onClick={() => setSelectedPeriod("year")}
            >
              Year
            </button>
          </div>

          {/* Financial Stats Overview */}
          <div class="stats stats-vertical lg:stats-horizontal shadow mb-8 w-full">
            <div class="stat">
              <div class="stat-figure text-success">
                <HiOutlineArrowTrendingUp class="w-8 h-8" />
              </div>
              <div class="stat-title">Total Income</div>
              <div class="stat-value text-success">
                ${summary()!.total_income.toLocaleString()}
              </div>
              <div class="stat-desc">
                {summary()!.period_start} to {summary()!.period_end}
              </div>
            </div>

            <div class="stat">
              <div class="stat-figure text-error">
                <HiOutlineArrowTrendingDown class="w-8 h-8" />
              </div>
              <div class="stat-title">Total Expenses</div>
              <div class="stat-value text-error">
                ${summary()!.total_expenses.toLocaleString()}
              </div>
              <div class="stat-desc">
                {Math.round(
                  (summary()!.total_expenses / summary()!.total_income) * 100,
                )}
                % of income
              </div>
            </div>

            <div class="stat">
              <div class="stat-figure text-primary">
                <HiOutlineCurrencyDollar class="w-8 h-8" />
              </div>
              <div class="stat-title">Net Balance</div>
              <div class="stat-value text-primary">
                ${summary()!.net_balance.toLocaleString()}
              </div>
              <div class="stat-desc">
                {summary()!.net_balance > 0 ? "Positive" : "Negative"} cash flow
              </div>
            </div>

            <div class="stat">
              <div class="stat-figure text-warning">
                <HiOutlineExclamationTriangle class="w-8 h-8" />
              </div>
              <div class="stat-title">Pending/Overdue</div>
              <div class="stat-value text-warning">
                {summary()!.pending_bills + summary()!.overdue_payments}
              </div>
              <div class="stat-desc">
                {summary()!.overdue_payments} overdue,{" "}
                {summary()!.pending_bills} pending
              </div>
            </div>
          </div>

          <div class="grid grid-cols-1 lg:grid-cols-3 gap-6 mb-8">
            {/* Recent Transactions */}
            <div class="lg:col-span-2">
              <div class="card bg-base-100 shadow-sm border border-base-300">
                <div class="card-body">
                  <div class="flex items-center justify-between mb-4">
                    <h2 class="card-title">Recent Transactions</h2>
                    <button class="btn btn-ghost btn-sm">View All</button>
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
                            <th>Description</th>
                            <th>Category</th>
                            <th>Amount</th>
                            <th>Status</th>
                          </tr>
                        </thead>
                        <tbody>
                          <For each={transactions()}>
                            {(transaction) => {
                              const CategoryIcon = getCategoryIcon(
                                transaction.category,
                              );
                              return (
                                <tr class="hover">
                                  <td class="text-xs">
                                    {new Date(
                                      transaction.transaction_date,
                                    ).toLocaleDateString()}
                                  </td>
                                  <td>
                                    <div class="flex items-center gap-2">
                                      <CategoryIcon class="w-4 h-4 flex-shrink-0" />
                                      <div>
                                        <div class="font-medium text-sm">
                                          {transaction.description}
                                        </div>
                                        {transaction.vendor && (
                                          <div class="text-xs text-base-content/60">
                                            {transaction.vendor}
                                          </div>
                                        )}
                                      </div>
                                    </div>
                                  </td>
                                  <td>
                                    <span class="badge badge-sm badge-outline">
                                      {transaction.category}
                                    </span>
                                  </td>
                                  <td
                                    classList={{
                                      "text-success font-semibold":
                                        transaction.amount > 0,
                                      "text-error": transaction.amount < 0,
                                    }}
                                  >
                                    {transaction.amount > 0 ? "+" : ""}$
                                    {Math.abs(
                                      transaction.amount,
                                    ).toLocaleString()}
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
                  </Show>
                </div>
              </div>
            </div>

            {/* Upcoming Bills */}
            <div>
              <div class="card bg-base-100 shadow-sm border border-base-300">
                <div class="card-body">
                  <h2 class="card-title mb-4">Upcoming Bills</h2>

                  <Show
                    when={upcomingBills().length > 0}
                    fallback={
                      <div class="text-center py-8 text-base-content/60">
                        No upcoming bills
                      </div>
                    }
                  >
                    <div class="space-y-3">
                      <For each={upcomingBills()}>
                        {(bill) => (
                          <div class="flex items-start gap-3 p-3 rounded-lg bg-base-200 hover:bg-base-300 transition-colors">
                            <div
                              class="w-10 h-10 rounded-full flex items-center justify-center flex-shrink-0"
                              classList={{
                                "bg-error/20 text-error":
                                  bill.status === "overdue",
                                "bg-warning/20 text-warning":
                                  bill.status === "pending",
                              }}
                            >
                              {bill.status === "overdue" ? (
                                <HiOutlineExclamationTriangle class="w-5 h-5" />
                              ) : (
                                <HiOutlineClock class="w-5 h-5" />
                              )}
                            </div>
                            <div class="flex-1 min-w-0">
                              <div class="font-medium text-sm">
                                {bill.description}
                              </div>
                              <div class="text-xs text-base-content/60 flex items-center gap-1 mt-1">
                                <HiOutlineCalendar class="w-3 h-3" />
                                Due{" "}
                                {new Date(
                                  bill.transaction_date,
                                ).toLocaleDateString()}
                              </div>
                              {bill.notes && (
                                <div class="text-xs text-error mt-1">
                                  {bill.notes}
                                </div>
                              )}
                            </div>
                            <div class="text-right">
                              <div class="font-semibold text-sm">
                                ${Math.abs(bill.amount).toLocaleString()}
                              </div>
                              <span
                                class={`badge badge-xs ${getStatusBadgeClass(bill.status)}`}
                              >
                                {bill.status}
                              </span>
                            </div>
                          </div>
                        )}
                      </For>
                    </div>

                    <button class="btn btn-sm btn-block mt-4">
                      View All Bills
                    </button>
                  </Show>
                </div>
              </div>
            </div>
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
                            ${cat.amount.toLocaleString()} ({cat.percentage}%)
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
              <h3 class="text-xl font-semibold mb-2">No financial data yet</h3>
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
  );
}
