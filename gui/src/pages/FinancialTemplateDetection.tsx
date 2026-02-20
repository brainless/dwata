import { A } from "@solidjs/router";
import { HiOutlineSparkles } from "solid-icons/hi";
import { Show, createSignal, onCleanup, onMount } from "solid-js";
import type { FinancialTemplateDetectionJobState } from "../api-types/types";
import { getApiUrl } from "../config/api";
import FinancialPageLayout from "../components/FinancialPageLayout";

async function fetchDetectJobState(): Promise<FinancialTemplateDetectionJobState> {
  const response = await fetch(getApiUrl("/api/financial/templates/detect"));
  if (!response.ok) throw new Error(`HTTP ${response.status}`);
  return await response.json();
}

async function startDetection(): Promise<FinancialTemplateDetectionJobState> {
  const response = await fetch(getApiUrl("/api/financial/templates/detect"), {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ max_senders: 3 }),
  });
  if (!response.ok) throw new Error(`HTTP ${response.status}`);
  return await response.json();
}

export default function FinancialTemplateDetection() {
  const [job, setJob] = createSignal<FinancialTemplateDetectionJobState | null>(null);
  const [loading, setLoading] = createSignal(true);
  const [submitting, setSubmitting] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);

  let timer: number | undefined;
  const load = async () => {
    try {
      const state = await fetchDetectJobState();
      setJob(state);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load detection status");
    } finally {
      setLoading(false);
    }
  };

  onMount(async () => {
    await load();
    timer = window.setInterval(load, 1500);
  });

  onCleanup(() => {
    if (timer) window.clearInterval(timer);
  });

  const onStart = async () => {
    setSubmitting(true);
    try {
      const state = await startDetection();
      setJob(state);
      setError(null);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to start detection");
    } finally {
      setSubmitting(false);
    }
  };

  const percent = () => {
    const state = job();
    if (!state || state.total_senders <= 0) return 0;
    return Math.min(100, Math.round((state.processed_senders / state.total_senders) * 100));
  };

  const footerActions = (
    <>
      <A href="/financial/templates" class="btn btn-ghost btn-sm">
        Back to Templates
      </A>
    </>
  );

  return (
    <FinancialPageLayout
      title="Financial Overview: Detect Templates"
      subtitle="Use AI to discover reusable email patterns for bills and transactions."
      footer={footerActions}
    >
      <div class="card bg-base-100 shadow">
        <div class="card-body">
          <div class="flex items-center gap-3">
            <HiOutlineSparkles class="w-6 h-6 text-primary" />
            <h2 class="card-title">Template Detection</h2>
          </div>
          <p class="text-sm text-base-content/70">We group similar emails and learn a reusable template.</p>
          <p class="text-sm text-base-content/70">Then we identify variable parts, like amounts and dates, so future emails can be linked automatically.</p>

          <Show when={error()}>
            <div class="alert alert-error mt-3">
              <span>{error()}</span>
            </div>
          </Show>

          <Show when={!loading()} fallback={<div class="mt-4 text-sm text-base-content/70">Loading detection status...</div>}>
            <Show
              when={job() && job()!.status !== "idle"}
              fallback={
                <div class="mt-4">
                  <button class="btn btn-primary" onClick={onStart} disabled={submitting()}>
                    {submitting() ? "Starting..." : "Detect Templates with AI"}
                  </button>
                </div>
              }
            >
              <div class="mt-4 space-y-3">
                <div class="text-sm">
                  <span class="font-medium">Status:</span> {job()!.status}
                </div>
                <div class="text-sm">
                  <span class="font-medium">Processed senders:</span> {job()!.processed_senders} / {job()!.total_senders}
                </div>
                <Show when={job()!.current_sender}>
                  <div class="text-sm">
                    <span class="font-medium">Current sender:</span> {job()!.current_sender}
                  </div>
                </Show>
                <Show when={job()!.status === "completed"}>
                  <div class="text-sm">
                    <span class="font-medium">New templates created:</span> {job()!.new_templates_count}
                  </div>
                </Show>
                <Show when={job()!.status === "failed" && job()!.error}>
                  <div class="text-sm text-error">{job()!.error}</div>
                </Show>
                <progress class="progress progress-primary w-full" max="100" value={percent()} />
              </div>
            </Show>
          </Show>
        </div>
      </div>
    </FinancialPageLayout>
  );
}
