import { A } from "@solidjs/router";
import { HiOutlineSparkles } from "solid-icons/hi";
import { For, Show, createMemo, createSignal, onCleanup, onMount } from "solid-js";
import type { FinancialTemplateDetectionJobState } from "../api-types/types";
import { getApiUrl } from "../config/api";
import FinancialPageLayout from "../components/FinancialPageLayout";

type DetectJobFetchResult = {
  state: FinancialTemplateDetectionJobState;
  version: number | null;
};

async function fetchDetectJobState(sinceVersion?: number): Promise<DetectJobFetchResult> {
  const params = new URLSearchParams();
  params.set("timeout_ms", "25000");
  if (sinceVersion !== undefined) {
    params.set("since_version", String(sinceVersion));
  }
  const response = await fetch(getApiUrl(`/api/financial/templates/detect?${params.toString()}`));
  if (!response.ok) throw new Error(`HTTP ${response.status}`);
  const versionHeader = response.headers.get("x-detect-state-version");
  const version = versionHeader ? Number(versionHeader) : null;
  return {
    state: await response.json(),
    version: Number.isFinite(version) ? version : null,
  };
}

async function startDetection(): Promise<FinancialTemplateDetectionJobState> {
  const response = await fetch(getApiUrl("/api/financial/templates/detect"), {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({}),
  });
  if (!response.ok) throw new Error(`HTTP ${response.status}`);
  return await response.json();
}

export default function FinancialTemplateDetection() {
  const [job, setJob] = createSignal<FinancialTemplateDetectionJobState | null>(null);
  const [loading, setLoading] = createSignal(true);
  const [submitting, setSubmitting] = createSignal(false);
  const [error, setError] = createSignal<string | null>(null);
  const [senderFilter, setSenderFilter] = createSignal("");
  const [stateVersion, setStateVersion] = createSignal<number | undefined>(undefined);

  let active = true;
  const sleep = (ms: number) => new Promise((resolve) => window.setTimeout(resolve, ms));
  const loadLoop = async () => {
    while (active) {
      try {
        const result = await fetchDetectJobState(stateVersion());
        setJob(result.state);
        if (result.version !== null) {
          setStateVersion(result.version);
        } else {
          // Fallback when response headers are not exposed/readable (e.g. CORS):
          // avoid tight-looping and degrade to short-interval polling.
          await sleep(1500);
        }
        setError(null);
      } catch (err) {
        setError(err instanceof Error ? err.message : "Failed to load detection status");
        await sleep(1500);
      } finally {
        setLoading(false);
      }
    }
  };

  onMount(async () => {
    void loadLoop();
  });

  onCleanup(() => {
    active = false;
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
  const debug = () => job()?.debug ?? null;
  const filteredSenderDebug = createMemo(() => {
    const filter = senderFilter().trim().toLowerCase();
    const senderRows = debug()?.sender_debug ?? [];
    if (!filter) return senderRows;
    return senderRows.filter((row) => row.sender_email.toLowerCase().includes(filter));
  });

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

          <Show when={job() && job()!.status === "completed"}>
            <div class="mt-5">
              <button class="btn btn-primary" onClick={onStart} disabled={submitting()}>
                {submitting() ? "Starting..." : "Detect Templates with AI"}
              </button>
            </div>
          </Show>

          <Show when={debug()}>
            <div class="divider my-5">Scan Details</div>
            <div class="space-y-5 text-sm">
              <div>
                <div><span class="font-medium">Keyword query:</span> {debug()!.keyword_query}</div>
                <div><span class="font-medium">Matched document IDs:</span> {debug()!.matched_document_ids_count}</div>
              </div>

              <div class="overflow-x-auto">
                <div class="font-medium mb-2">Sender ranking</div>
                <table class="table table-sm">
                  <thead>
                    <tr>
                      <th>#</th>
                      <th>Sender</th>
                      <th>Total</th>
                      <th>Recent</th>
                      <th>Max Existing Cluster</th>
                    </tr>
                  </thead>
                  <tbody>
                    <For each={debug()!.sender_ranking}>
                      {(r) => (
                        <tr>
                          <td>{r.rank}</td>
                          <td>{r.sender_email}</td>
                          <td>{r.total_candidate_emails}</td>
                          <td>{r.recent_candidate_emails}</td>
                          <td>{r.max_existing_cluster_size}</td>
                        </tr>
                      )}
                    </For>
                  </tbody>
                </table>
              </div>

              <div class="overflow-x-auto">
                <div class="font-medium mb-2">Candidate email previews (first {debug()!.candidate_email_previews.length})</div>
                <table class="table table-sm">
                  <thead>
                    <tr>
                      <th>Sender</th>
                      <th>Subject</th>
                      <th>Preview</th>
                    </tr>
                  </thead>
                  <tbody>
                    <For each={debug()!.candidate_email_previews}>
                      {(row) => (
                        <tr>
                          <td>{row.sender_email}</td>
                          <td>{row.subject}</td>
                          <td class="max-w-[420px] whitespace-pre-wrap">{row.body_preview}</td>
                        </tr>
                      )}
                    </For>
                  </tbody>
                </table>
              </div>

              <div>
                <div class="font-medium mb-2">Sender execution details</div>
                <input
                  class="input input-bordered input-sm w-full md:w-80"
                  placeholder="Filter sender email..."
                  value={senderFilter()}
                  onInput={(e) => setSenderFilter(e.currentTarget.value)}
                />
                <div class="space-y-3 mt-3">
                  <For each={filteredSenderDebug()}>
                    {(s) => (
                      <div class="border border-base-300 rounded p-3">
                        <div class="font-medium">{s.rank}. {s.sender_email}</div>
                        <div>candidates={s.sender_candidate_count}, existing_templates={s.existing_template_count}, initial_matches={s.initially_matched_count}, unmatched={s.fresh_unmatched_count}, pool={s.pool_count}</div>
                        <Show when={s.skipped_reason}><div class="text-warning">skipped: {s.skipped_reason}</div></Show>
                        <Show when={s.error}><div class="text-error">error: {s.error}</div></Show>
                        <Show when={s.generated_templates.length > 0}>
                          <div class="mt-2 space-y-2">
                            <For each={s.generated_templates}>
                              {(t) => (
                                <div class="bg-base-200 rounded p-2">
                                  <div>template_id={t.template_id ? String(t.template_id) : "none"} type={t.template_type ?? "n/a"} {t.discarded_reason ? `(discarded: ${t.discarded_reason})` : ""}</div>
                                  <div class="text-xs whitespace-pre-wrap mt-1">{t.template_body}</div>
                                </div>
                              )}
                            </For>
                          </div>
                        </Show>
                      </div>
                    )}
                  </For>
                </div>
              </div>
            </div>
          </Show>
        </div>
      </div>
    </FinancialPageLayout>
  );
}
