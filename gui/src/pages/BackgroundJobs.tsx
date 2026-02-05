import { createSignal, onMount, For, Show } from "solid-js";
import {
  HiOutlinePause,
  HiOutlineTrash,
  HiOutlineCloudArrowDown,
  HiOutlineDocumentText,
} from "solid-icons/hi";
import { getApiUrl } from "../config/api";
import type {
  DownloadJob,
  DownloadJobStatus,
  ExtractionJob,
  ExtractionJobStatus,
  CredentialMetadata,
  SourceType,
} from "../api-types/types";

type Tab = "downloads" | "extractions";

export default function BackgroundJobs() {
  const [activeTab, setActiveTab] = createSignal<Tab>("downloads");
  const [downloadJobs, setDownloadJobs] = createSignal<DownloadJob[]>([]);
  const [extractionJobs, setExtractionJobs] = createSignal<ExtractionJob[]>([]);
  const [credentials, setCredentials] = createSignal<CredentialMetadata[]>([]);
  const [loading, setLoading] = createSignal(false);
  const [startFormLoading, setStartFormLoading] = createSignal(false);

  const [selectedDownloads, setSelectedDownloads] = createSignal<Set<string>>(
    new Set(),
  );
  const [selectedExtractions, setSelectedExtractions] = createSignal<
    Set<number>
  >(new Set());

  const fetchDownloadJobs = async () => {
    try {
      setLoading(true);
      const response = await fetch(getApiUrl("/api/downloads"));
      const data = await response.json();
      setDownloadJobs(data.jobs);
    } catch (error) {
      console.error("Failed to fetch download jobs:", error);
    } finally {
      setLoading(false);
    }
  };

  const fetchExtractionJobs = async () => {
    try {
      setLoading(true);
      const response = await fetch(getApiUrl("/api/extractions"));
      const data = await response.json();
      setExtractionJobs(data.jobs);
    } catch (error) {
      console.error("Failed to fetch extraction jobs:", error);
    } finally {
      setLoading(false);
    }
  };

  const fetchCredentials = async () => {
    try {
      const response = await fetch(getApiUrl("/api/credentials"));
      const data = await response.json();
      setCredentials(data.credentials);
    } catch (error) {
      console.error("Failed to fetch credentials:", error);
    }
  };

  const pauseDownloadJob = async (id: number) => {
    try {
      await fetch(getApiUrl(`/api/downloads/${id}/pause`), { method: "POST" });
      await fetchDownloadJobs();
    } catch (error) {
      console.error("Failed to pause download job:", error);
    }
  };

  const deleteDownloadJob = async (id: number) => {
    try {
      await fetch(getApiUrl(`/api/downloads/${id}`), { method: "DELETE" });
      await fetchDownloadJobs();
    } catch (error) {
      console.error("Failed to delete download job:", error);
    }
  };

  const startSelectedJobs = async () => {
    try {
      setStartFormLoading(true);
      const tab = activeTab();

      if (tab === "downloads") {
        const selected = selectedDownloads();
        for (const key of selected) {
          const [credentialId, sourceType] = key.split(":");

          const requestBody = {
            credential_id: Number(credentialId),
            source_type: sourceType,
          };
          console.log(
            "Sending request body:",
            JSON.stringify(requestBody, null, 2),
          );

          const response = await fetch(getApiUrl("/api/downloads"), {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify(requestBody),
          });

          if (!response.ok) {
            const errorText = await response.text();
            console.error(
              "Failed to create download job:",
              response.status,
              errorText,
            );
            continue;
          }

          const job = await response.json();
          await fetch(getApiUrl(`/api/downloads/${job.id}/start`), {
            method: "POST",
          });
        }
        setSelectedDownloads(new Set());
        await fetchDownloadJobs();
      } else {
        const selected = selectedExtractions();
        for (const credentialId of selected) {
          await fetch(getApiUrl("/api/financial/extract"), {
            method: "POST",
            headers: { "Content-Type": "application/json" },
            body: JSON.stringify({
              credential_id: credentialId,
            }),
          });
        }
        setSelectedExtractions(new Set());
      }
    } catch (error) {
      console.error("Failed to start jobs:", error);
    } finally {
      setStartFormLoading(false);
    }
  };

  onMount(() => {
    fetchDownloadJobs();
    fetchExtractionJobs();
    fetchCredentials();
  });

  const formatDate = (timestamp: bigint | null) => {
    if (!timestamp) return "N/A";
    return new Date(Number(timestamp)).toLocaleString();
  };

  const getStatusBadgeClass = (status: string) => {
    switch (status) {
      case "running":
        return "badge-info";
      case "completed":
        return "badge-success";
      case "failed":
        return "badge-error";
      case "paused":
        return "badge-warning";
      case "pending":
        return "badge-ghost";
      case "cancelled":
        return "badge-error";
      default:
        return "badge-ghost";
    }
  };

  const currentDownloadJobs = () => downloadJobs();
  const currentExtractionJobs = () => extractionJobs();

  // Map credential type to its applicable source type
  const getSourceTypeForCredential = (
    credential: CredentialMetadata,
  ): SourceType | null => {
    switch (credential.credential_type) {
      case "imap":
        // IMAP credentials (whether using OAuth2 or plain auth) are for IMAP downloads
        return "imap";
      case "oauth":
        // Pure OAuth credentials for cloud storage
        // TODO: Determine OAuth provider from credential metadata
        // For now, assume Google Drive
        return "google-drive";
      default:
        // Other credential types (smtp, apikey, database, localfile, custom)
        // don't have associated download source types
        return null;
    }
  };

  return (
    <div class="p-4 md:p-8">
      <div class="flex flex-col md:flex-row md:items-center md:justify-between mb-8">
        <div>
          <h1 class="text-3xl font-bold mb-2">Background Jobs</h1>
          <p class="text-base-content/60">
            Monitor and manage download and extraction jobs
          </p>
        </div>
      </div>

      <div class="tabs tabs-boxed mb-6">
        <button
          class={`tab tab-lg ${activeTab() === "downloads" ? "tab-active" : ""}`}
          onClick={() => setActiveTab("downloads")}
        >
          <HiOutlineCloudArrowDown class="w-5 h-5 mr-2" />
          Downloads ({currentDownloadJobs().length})
        </button>
        <button
          class={`tab tab-lg ${activeTab() === "extractions" ? "tab-active" : ""}`}
          onClick={() => setActiveTab("extractions")}
        >
          <HiOutlineDocumentText class="w-5 h-5 mr-2" />
          Extractions ({currentExtractionJobs().length})
        </button>
      </div>

      <Show when={loading()}>
        <div class="flex justify-center py-8">
          <span class="loading loading-spinner loading-lg"></span>
        </div>
      </Show>

      <Show when={!loading()}>
        <Show when={activeTab() === "downloads"}>
          <DownloadJobsTable
            jobs={currentDownloadJobs()}
            onPause={pauseDownloadJob}
            onDelete={deleteDownloadJob}
            formatDate={formatDate}
            getStatusBadgeClass={getStatusBadgeClass}
          />
          <StartDownloadsForm
            credentials={credentials()}
            selectedDownloads={selectedDownloads()}
            onSelectionChange={setSelectedDownloads}
            onStart={startSelectedJobs}
            loading={startFormLoading()}
            getSourceTypeForCredential={getSourceTypeForCredential}
          />
        </Show>

        <Show when={activeTab() === "extractions"}>
          <ExtractionJobsTable
            jobs={currentExtractionJobs()}
            formatDate={formatDate}
            getStatusBadgeClass={getStatusBadgeClass}
          />
          <StartExtractionsForm
            credentials={credentials()}
            selectedExtractions={selectedExtractions()}
            onSelectionChange={setSelectedExtractions}
            onStart={startSelectedJobs}
            loading={startFormLoading()}
          />
        </Show>
      </Show>
    </div>
  );
}

function DownloadJobsTable(props: {
  jobs: DownloadJob[];
  onPause: (id: number) => void;
  onDelete: (id: number) => void;
  formatDate: (timestamp: bigint | null) => string;
  getStatusBadgeClass: (status: string) => string;
}) {
  return (
    <div class="overflow-x-auto mb-8">
      <table class="table table-zebra">
        <thead>
          <tr>
            <th>ID</th>
            <th>Type</th>
            <th>Status</th>
            <th>Downloaded</th>
            <th>Created</th>
            <th>Actions</th>
          </tr>
        </thead>
        <tbody>
          <For each={props.jobs}>
            {(job) => (
              <tr>
                <td>{job.id}</td>
                <td>{job.source_type}</td>
                <td>
                  <span
                    class={`badge ${props.getStatusBadgeClass(job.status)}`}
                  >
                    {job.status}
                  </span>
                </td>
                <td>
                  <div class="text-sm">
                    {job.progress.downloaded_items} / {job.progress.total_items}
                  </div>
                </td>
                <td class="text-sm">{props.formatDate(job.created_at)}</td>
                <td>
                  <div class="flex gap-1">
                    <Show when={job.status === "running"}>
                      <button
                        class="btn btn-ghost btn-sm"
                        onClick={() => props.onPause(Number(job.id))}
                        title="Pause"
                      >
                        <HiOutlinePause class="w-4 h-4" />
                      </button>
                    </Show>
                    <button
                      class="btn btn-ghost btn-sm text-error"
                      onClick={() => props.onDelete(Number(job.id))}
                      title="Delete"
                    >
                      <HiOutlineTrash class="w-4 h-4" />
                    </button>
                  </div>
                </td>
              </tr>
            )}
          </For>
        </tbody>
      </table>

      <Show when={props.jobs.length === 0}>
        <div class="text-center py-12 text-base-content/60">
          No download jobs found
        </div>
      </Show>
    </div>
  );
}

function ExtractionJobsTable(props: {
  jobs: ExtractionJob[];
  formatDate: (timestamp: bigint | null) => string;
  getStatusBadgeClass: (status: string) => string;
}) {
  return (
    <div class="overflow-x-auto mb-8">
      <table class="table table-zebra">
        <thead>
          <tr>
            <th>ID</th>
            <th>Source Type</th>
            <th>Extractor Type</th>
            <th>Status</th>
            <th>Created</th>
          </tr>
        </thead>
        <tbody>
          <For each={props.jobs}>
            {(job) => (
              <tr>
                <td>{job.id}</td>
                <td>{job.source_type}</td>
                <td>{job.extractor_type}</td>
                <td>
                  <span
                    class={`badge ${props.getStatusBadgeClass(job.status)}`}
                  >
                    {job.status}
                  </span>
                </td>
                <td class="text-sm">{props.formatDate(job.created_at)}</td>
              </tr>
            )}
          </For>
        </tbody>
      </table>

      <Show when={props.jobs.length === 0}>
        <div class="text-center py-12 text-base-content/60">
          No extraction jobs found
        </div>
      </Show>
    </div>
  );
}

function StartDownloadsForm(props: {
  credentials: CredentialMetadata[];
  selectedDownloads: Set<string>;
  onSelectionChange: (selected: Set<string>) => void;
  onStart: () => void;
  loading: boolean;
  getSourceTypeForCredential: (
    credential: CredentialMetadata,
  ) => SourceType | null;
}) {
  const handleCheckboxChange = (key: string, checked: boolean) => {
    props.onSelectionChange((prev) => {
      const next = new Set(prev);
      if (checked) {
        next.add(key);
      } else {
        next.delete(key);
      }
      return next;
    });
  };

  const handleSelectAll = (checked: boolean) => {
    const allKeys: string[] = [];
    for (const cred of props.credentials) {
      const sourceType = props.getSourceTypeForCredential(cred);
      if (sourceType) {
        allKeys.push(`${cred.id}:${sourceType}`);
      }
    }
    props.onSelectionChange(new Set(checked ? allKeys : []));
  };

  const allKeys = () => {
    const keys: string[] = [];
    for (const cred of props.credentials) {
      const sourceType = props.getSourceTypeForCredential(cred);
      if (sourceType) {
        keys.push(`${cred.id}:${sourceType}`);
      }
    }
    return keys;
  };

  const allSelected = () =>
    allKeys().length > 0 &&
    allKeys().every((key) => props.selectedDownloads.has(key));

  return (
    <div class="card bg-base-100 shadow-sm border border-base-300">
      <div class="card-body">
        <h2 class="card-title mb-4">Start New Download Job(s)</h2>

        <Show
          when={props.credentials.length > 0}
          fallback={
            <p class="text-base-content/60">
              No credentials available. Add credentials first to start download
              jobs.
            </p>
          }
        >
          <div class="overflow-x-auto">
            <table class="table table-sm">
              <thead>
                <tr>
                  <th>
                    <input
                      type="checkbox"
                      class="checkbox checkbox-sm"
                      checked={allSelected()}
                      onChange={(e) => handleSelectAll(e.currentTarget.checked)}
                    />
                  </th>
                  <th>Account</th>
                  <th>Source Type</th>
                </tr>
              </thead>
              <tbody>
                <For each={props.credentials}>
                  {(cred) => {
                    const sourceType = props.getSourceTypeForCredential(cred);
                    if (!sourceType) return null;

                    const key = `${cred.id}:${sourceType}`;
                    return (
                      <tr>
                        <td>
                          <input
                            type="checkbox"
                            class="checkbox checkbox-sm"
                            checked={props.selectedDownloads.has(key)}
                            onChange={(e) =>
                              handleCheckboxChange(key, e.currentTarget.checked)
                            }
                          />
                        </td>
                        <td>{cred.identifier}</td>
                        <td>{sourceType}</td>
                      </tr>
                    );
                  }}
                </For>
              </tbody>
            </table>
          </div>

          <div class="card-actions justify-end mt-4">
            <button
              class="btn btn-primary"
              onClick={props.onStart}
              disabled={props.selectedDownloads.size === 0 || props.loading}
            >
              {props.loading ? (
                <span class="loading loading-spinner loading-sm"></span>
              ) : (
                "Start Selected Jobs"
              )}
            </button>
          </div>
        </Show>
      </div>
    </div>
  );
}

function StartExtractionsForm(props: {
  credentials: CredentialMetadata[];
  selectedExtractions: Set<number>;
  onSelectionChange: (selected: Set<number>) => void;
  onStart: () => void;
  loading: boolean;
}) {
  const handleCheckboxChange = (id: number, checked: boolean) => {
    props.onSelectionChange((prev) => {
      const next = new Set(prev);
      if (checked) {
        next.add(id);
      } else {
        next.delete(id);
      }
      return next;
    });
  };

  const handleSelectAll = (checked: boolean) => {
    const allIds: number[] = [];
    for (const cred of props.credentials) {
      if (cred.credential_type === "imap") {
        allIds.push(Number(cred.id));
      }
    }
    props.onSelectionChange(new Set(checked ? allIds : []));
  };

  const allIds = () => {
    const ids: number[] = [];
    for (const cred of props.credentials) {
      if (cred.credential_type === "imap") {
        ids.push(Number(cred.id));
      }
    }
    return ids;
  };

  const allSelected = () =>
    allIds().length > 0 &&
    allIds().every((id) => props.selectedExtractions.has(id));

  const emailCredentials = () => {
    const filtered = props.credentials.filter((cred) => {
      const isImapType = cred.credential_type === "imap";
      const isImapOauth =
        cred.credential_type === "oauth" && cred.service_name?.includes("imap");
      return isImapType || isImapOauth;
    });
    return filtered;
  };

  return (
    <div class="card bg-base-100 shadow-sm border border-base-300">
      <div class="card-body">
        <h2 class="card-title mb-4">Start New Extraction Job(s)</h2>

        <Show
          when={emailCredentials().length > 0}
          fallback={
            <p class="text-base-content/60">
              No IMAP email accounts available. Add IMAP credentials first to
              run financial extraction.
            </p>
          }
        >
          <div class="overflow-x-auto">
            <table class="table table-sm">
              <thead>
                <tr>
                  <th>
                    <input
                      type="checkbox"
                      class="checkbox checkbox-sm"
                      checked={allSelected()}
                      onChange={(e) => handleSelectAll(e.currentTarget.checked)}
                    />
                  </th>
                  <th>Email Account</th>
                </tr>
              </thead>
              <tbody>
                <For each={emailCredentials()}>
                  {(cred) => (
                    <tr>
                      <td>
                        <input
                          type="checkbox"
                          class="checkbox checkbox-sm"
                          checked={props.selectedExtractions.has(
                            Number(cred.id),
                          )}
                          onChange={(e) =>
                            handleCheckboxChange(
                              Number(cred.id),
                              e.currentTarget.checked,
                            )
                          }
                        />
                      </td>
                      <td>{cred.identifier}</td>
                    </tr>
                  )}
                </For>
              </tbody>
            </table>
          </div>

          <div class="card-actions justify-end mt-4">
            <button
              class="btn btn-primary"
              onClick={props.onStart}
              disabled={props.selectedExtractions.size === 0 || props.loading}
            >
              {props.loading ? (
                <span class="loading loading-spinner loading-sm"></span>
              ) : (
                "Start Selected Jobs"
              )}
            </button>
          </div>
        </Show>
      </div>
    </div>
  );
}
