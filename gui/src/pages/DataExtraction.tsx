import { createSignal, onCleanup, onMount, Show, For } from "solid-js";
import { getApiUrl } from "../config/api";
import type { ExtractionStepState, ExtractionStep, PassStepState, ExtractionSummary, LabelDocumentParams } from "../api-types/types";

interface AccountProgress {
  credential_id: number;
  identifier: string;
  status: string;
  total_emails: number;
  emails_processed: number;
  emails_failed: number;
  current_email_id: number | null;
  current_session_id: number | null;
  started_at: number | null;
  completed_at: number | null;
  error_message: string | null;
}

interface ProgressResponse {
  active: boolean;
  accounts: AccountProgress[];
  updated: boolean;
}

export default function DataExtraction() {
  const [isDeleting, setIsDeleting] = createSignal(false);
  const [deleteError, setDeleteError] = createSignal("");
  const [deleteSuccess, setDeleteSuccess] = createSignal("");
  
  // Extraction state
  const [isExtracting, setIsExtracting] = createSignal(false);
  const [extractionError, setExtractionError] = createSignal("");
  const [progress, setProgress] = createSignal<ProgressResponse | null>(null);
  const [isPolling, setIsPolling] = createSignal(false);
  
  // Step-wise extraction state
  const [stepState, setStepState] = createSignal<ExtractionStepState | null>(null);
  const [isLoadingStepState, setIsLoadingStepState] = createSignal(false);
  
  let abortController: AbortController | null = null;

  const startExtraction = async () => {
    setIsExtracting(true);
    setExtractionError("");
    setProgress(null);

    try {
      const response = await fetch(getApiUrl("/api/kg-extraction/run"), {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({}),
      });

      if (!response.ok) {
        throw new Error(`Failed to start extraction: ${response.statusText}`);
      }

      const data = await response.json();
      console.log("Extraction started:", data);
      
      // Start long polling
      startLongPolling();
    } catch (error) {
      setExtractionError(error instanceof Error ? error.message : "An error occurred");
      setIsExtracting(false);
      setIsPolling(false);
    }
  };

  const fetchProgressLongPoll = async () => {
    // Create new abort controller for this request
    abortController = new AbortController();
    
    try {
      const response = await fetch(
        getApiUrl("/api/kg-extraction/progress?long_poll=true"),
        { signal: abortController.signal }
      );
      
      if (!response.ok) {
        console.error("Failed to fetch progress:", response.statusText);
        return false;
      }
      
      const data: ProgressResponse = await response.json();
      setProgress(data);
      
      // Update step state for running sessions
      updateStepStateFromProgress();
      
      // Check if all accounts are done
      const allDone = data.accounts.length === 0 || data.accounts.every(
        (a) => a.status === "completed" || a.status === "failed" || a.status === "idle"
      );
      
      // Stop if extraction is not active and all accounts are done
      if (!data.active && allDone) {
        setIsExtracting(false);
        setIsPolling(false);
        return false;
      }
      
      return true;
    } catch (error) {
      if (error instanceof DOMException && error.name === "AbortError") {
        // Request was aborted (cleanup), stop polling
        return false;
      }
      console.error("Error fetching progress:", error);
      // Continue polling on other errors
      return true;
    } finally {
      abortController = null;
    }
  };

  const startLongPolling = async () => {
    setIsPolling(true);
    
    // Keep polling loop running while extracting
    while (isExtracting()) {
      const shouldContinue = await fetchProgressLongPoll();
      if (!shouldContinue) {
        break;
      }
      // Small delay before next long poll to prevent tight loops
      await new Promise(resolve => setTimeout(resolve, 100));
    }
    
    setIsPolling(false);
  };

  const stopPolling = () => {
    if (abortController) {
      abortController.abort();
      abortController = null;
    }
  };

  // Fetch detailed step state for a session
  const fetchStepState = async (sessionId: number) => {
    try {
      setIsLoadingStepState(true);
      const response = await fetch(getApiUrl(`/api/kg-extraction/step-state?session_id=${sessionId}`));
      
      if (!response.ok) {
        console.error("Failed to fetch step state:", response.statusText);
        return;
      }
      
      const data = await response.json();
      if (data.extraction_state) {
        setStepState(data.extraction_state);
      }
    } catch (error) {
      console.error("Error fetching step state:", error);
    } finally {
      setIsLoadingStepState(false);
    }
  };

  // Check if any account has a current session and fetch its state
  const updateStepStateFromProgress = () => {
    const currentProgress = progress();
    if (!currentProgress?.accounts) return;
    
    for (const account of currentProgress.accounts) {
      if (account.current_session_id && account.status === "running") {
        fetchStepState(account.current_session_id);
        return; // Only show one at a time
      }
    }
    
    // No running session found, clear step state
    setStepState(null);
  };

  onMount(async () => {
    try {
      const response = await fetch(getApiUrl("/api/kg-extraction/progress"));
      if (response.ok) {
        const data: ProgressResponse = await response.json();
        setProgress(data);
        if (data.active) {
          setIsExtracting(true);
          startLongPolling();
        }
      }
    } catch {
      // Ignore mount-time errors silently
    }
  });

  onCleanup(() => {
    stopPolling();
  });

  const handleDeleteConfirm = async () => {
    setIsDeleting(true);
    setDeleteError("");
    setDeleteSuccess("");

    try {
      const response = await fetch(getApiUrl("/api/clear-extracted-data"), {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
      });

      if (!response.ok) {
        throw new Error(`Failed to delete extracted data: ${response.statusText}`);
      }

      const data = await response.json();
      setDeleteSuccess(`Successfully cleared ${data.tables_cleared} tables.`);
      
      // Close modal after success
      setTimeout(() => {
        const modalToggle = document.getElementById("delete-modal-toggle") as HTMLInputElement;
        if (modalToggle) {
          modalToggle.checked = false;
        }
        setDeleteSuccess("");
      }, 2000);
    } catch (error) {
      setDeleteError(error instanceof Error ? error.message : "An error occurred");
    } finally {
      setIsDeleting(false);
    }
  };

  // Helper components for step-wise data display
  const DocumentLabelBadge = (props: { label: LabelDocumentParams }) => {
    const gates = [];
    if (props.label.has_bill) gates.push("Bill");
    if (props.label.has_transaction) gates.push("Transaction");
    if (props.label.has_event) gates.push("Event");
    if (props.label.has_order) gates.push("Order");
    
    return (
      <div class="flex flex-wrap gap-2">
        <span class="badge badge-primary">{props.label.doc_type}</span>
        {gates.map(gate => (
          <span class="badge badge-secondary badge-sm">{gate}</span>
        ))}
      </div>
    );
  };

  const StepTimelineItem = (props: { step: ExtractionStep; index: number }) => {
    const step = props.step;
    const time = new Date(Number(step.timestamp) * 1000).toLocaleTimeString();
    
    return (
      <div class="relative pl-8 pb-4 last:pb-0">
        <div class="absolute left-0 top-0 w-4 h-4 rounded-full bg-primary mt-1"></div>
        <div class="text-sm text-gray-500">{time}</div>
        <div class="font-medium">
          {step.step_type === "document_labeled" && "Document Labeled"}
          {step.step_type === "pass_started" && `Pass Started: ${step.pass_name}`}
          {step.step_type === "search_performed" && `Search Performed (${step.result_count} results)`}
          {step.step_type === "sender_search_performed" && `Sender Search (${step.result_count} results)`}
          {step.step_type === "entities_extracted" && `Entities Extracted (${step.total_entities} total)`}
          {step.step_type === "pass_completed" && "Pass Completed"}
          {step.step_type === "pass_failed" && "Pass Failed"}
          {step.step_type === "tool_call_made" && `Tool Call: ${step.tool_name}`}
          {step.step_type === "retry_occurred" && `Retry: ${step.reason}`}
        </div>
        
        {step.step_type === "document_labeled" && (
          <DocumentLabelBadge label={step.label} />
        )}
        
        {step.step_type === "entities_extracted" && (
          <div class="text-sm mt-1">
            <div class="flex flex-wrap gap-1">
              {Object.entries(step.entity_counts).map(([type, count]) => (
                <span class="badge badge-outline badge-sm">{type}: {count}</span>
              ))}
            </div>
          </div>
        )}
        
        {step.step_type === "pass_failed" && (
          <div class="text-error text-sm">{step.error_message}</div>
        )}
      </div>
    );
  };

  const PassStateCard = (props: { passKey: string; passState: PassStepState }) => {
    const state = props.passState;
    const statusColors = {
      pending: "badge-ghost",
      running: "badge-primary",
      completed: "badge-success",
      failed: "badge-error"
    };
    
    return (
      <div class="card bg-base-100 shadow-sm border border-base-300">
        <div class="card-body p-4">
          <div class="flex justify-between items-start">
            <h4 class="font-semibold capitalize">{state.pass_name.replace(/_/g, " ")}</h4>
            <span class={`badge ${statusColors[state.status]}`}>{state.status}</span>
          </div>
          
          {state.search_results.length > 0 && (
            <div class="text-sm text-gray-600 mt-2">
              {state.search_results.length} pre-populated entities
            </div>
          )}
          
          {state.error_message && (
            <div class="text-error text-sm mt-2">{state.error_message}</div>
          )}
        </div>
      </div>
    );
  };

  const ExtractionDetailPanel = () => {
    const state = stepState();
    if (!state) return null;
    
    const summary = state.summary;
    
    return (
      <div class="mt-6">
        <h3 class="text-lg font-semibold mb-4">Current Email Extraction</h3>
        
        {/* Extraction Summary */}
        <div class="card bg-base-200 mb-4">
          <div class="card-body p-4">
            <div class="flex justify-between items-center mb-2">
              <div>
                <span class="font-medium">Email ID:</span> {Number(summary.email_id)}
              </div>
              <span class={`badge ${summary.status === "running" ? "badge-primary" : summary.status === "completed" ? "badge-success" : "badge-error"}`}>
                {summary.status}
              </span>
            </div>
            
            {summary.sender_email && (
              <div class="text-sm text-gray-600 mb-2">From: {summary.sender_email}</div>
            )}
            
            <div class="grid grid-cols-3 gap-4 mt-4">
              <div class="stat bg-base-100 rounded-lg p-3">
                <div class="stat-value text-lg">{summary.completed_passes}/{summary.total_passes}</div>
                <div class="stat-title text-xs">Passes</div>
              </div>
              <div class="stat bg-base-100 rounded-lg p-3">
                <div class="stat-value text-lg">{summary.total_entities_extracted}</div>
                <div class="stat-title text-xs">Entities</div>
              </div>
              <div class="stat bg-base-100 rounded-lg p-3">
                <div class="stat-value text-lg">{summary.total_search_results}</div>
                <div class="stat-title text-xs">Searches</div>
              </div>
            </div>
          </div>
        </div>
        
        {/* Pass States */}
        <div class="mb-4">
          <h4 class="font-medium mb-2">Extraction Passes</h4>
          <div class="grid grid-cols-1 md:grid-cols-2 gap-3">
            <For each={Object.entries(state.pass_states)}>
              {([key, passState]) => <PassStateCard passKey={key} passState={passState} />}
            </For>
          </div>
        </div>
        
        {/* Steps Timeline */}
        <div class="card bg-base-200">
          <div class="card-body p-4">
            <h4 class="font-medium mb-4">Extraction Steps</h4>
            <div class="border-l-2 border-primary ml-2">
              <For each={state.steps}>
                {(step, index) => <StepTimelineItem step={step} index={index()} />}
              </For>
            </div>
          </div>
        </div>
      </div>
    );
  };

  return (
    <div class="data-extraction-page p-8 h-full min-h-0 overflow-y-auto">
      <h1 class="text-3xl font-bold mb-6">Data Extraction</h1>
      
      <div class="flex gap-4 mb-8">
        <button 
          class="btn btn-primary" 
          onClick={startExtraction}
          disabled={isExtracting()}
        >
          {isExtracting() ? (
            <>
              <span class="loading loading-spinner loading-sm mr-2"></span>
              {isPolling() ? "Processing..." : "Starting..."}
            </>
          ) : (
            "Start Extraction"
          )}
        </button>
        
        <label for="delete-modal-toggle" class="btn btn-error">
          Delete Extracted Data
        </label>
      </div>

      {/* Extraction Progress Display */}
      {(isExtracting() || progress()) && (
        <div class="mb-8">
          <h2 class="text-xl font-semibold mb-4">
            Extraction Progress
            {isPolling() && (
              <span class="ml-2 text-sm font-normal text-gray-500">
                (Live - Long Polling)
              </span>
            )}
          </h2>
          
          {extractionError() && (
            <div class="alert alert-error mb-4">
              <svg xmlns="http://www.w3.org/2000/svg" class="stroke-current shrink-0 h-6 w-6" fill="none" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2m7-2a9 9 0 11-18 0 9 9 0 0118 0z" />
              </svg>
              <span>{extractionError()}</span>
            </div>
          )}
          
          {progress()?.accounts.length === 0 ? (
            <div class="flex items-center text-gray-500">
              <span class="loading loading-spinner loading-sm mr-2"></span>
              Initializing...
            </div>
          ) : (
            <div class="space-y-4">
              {progress()?.accounts.map((account) => (
                <div class="card bg-base-200 p-4">
                  <div class="flex justify-between items-center mb-2">
                    <h3 class="font-semibold">{account.identifier}</h3>
                    <span class={`badge ${
                      account.status === "running" ? "badge-primary" :
                      account.status === "completed" ? "badge-success" :
                      account.status === "failed" ? "badge-error" :
                      "badge-ghost"
                    }`}>
                      {account.status}
                    </span>
                  </div>
                  
                  <div class="text-sm text-gray-600 mb-2">
                    {account.emails_processed} / {account.total_emails} emails processed
                    {account.emails_failed > 0 && (
                      <span class="text-error ml-2">({account.emails_failed} failed)</span>
                    )}
                  </div>
                  
                  {account.total_emails > 0 && (
                    <div class="w-full bg-gray-300 rounded-full h-2">
                      <div 
                        class={`h-2 rounded-full transition-all duration-300 ${
                          account.status === "completed" ? "bg-success" :
                          account.status === "failed" ? "bg-error" :
                          "bg-primary"
                        }`}
                        style={`width: ${Math.min(100, (account.emails_processed / account.total_emails) * 100)}%`}
                      ></div>
                    </div>
                  )}
                  
                  {account.error_message && (
                    <div class="text-error text-sm mt-2">{account.error_message}</div>
                  )}
                </div>
              ))}
            </div>
          )}
        </div>
      )}

      {/* Step-wise Extraction Details */}
      <Show when={stepState()}>
        <ExtractionDetailPanel />
      </Show>

      {/* Modal using Daisy UI modal with checkbox toggle */}
      <input type="checkbox" id="delete-modal-toggle" class="modal-toggle" />
      <div class="modal" role="dialog" aria-modal="true">
        <div class="modal-box">
          <h3 class="text-lg font-bold mb-4">Delete Extracted Data</h3>
          
          <div class="space-y-4">
            <div class="alert alert-warning">
              <svg xmlns="http://www.w3.org/2000/svg" class="stroke-current shrink-0 h-6 w-6" fill="none" viewBox="0 0 24 24">
                <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z" />
              </svg>
              <span>This action will delete all extracted data. This cannot be undone.</span>
            </div>

            <div>
              <h4 class="font-semibold mb-2">The following data will be deleted:</h4>
              <ul class="list-disc list-inside text-sm space-y-1 text-error">
                <li>Bills and invoices</li>
                <li>Financial transactions</li>
                <li>Organisations and companies</li>
                <li>Persons and contacts</li>
                <li>Orders</li>
                <li>Subscriptions</li>
                <li>Locations</li>
                <li>Events</li>
                <li>Contact links</li>
                <li>Knowledge graph entities</li>
              </ul>
            </div>

            <div>
              <h4 class="font-semibold mb-2">The following will be preserved:</h4>
              <ul class="list-disc list-inside text-sm space-y-1 text-success">
                <li>Email credentials and accounts</li>
                <li>Emails and email content</li>
                <li>Email folders</li>
                <li>Email labels and associations</li>
                <li>Email attachments</li>
              </ul>
            </div>

            {deleteError() && (
              <div class="alert alert-error">
                <svg xmlns="http://www.w3.org/2000/svg" class="stroke-current shrink-0 h-6 w-6" fill="none" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M10 14l2-2m0 0l2-2m-2 2l-2-2m2 2l2 2m7-2a9 9 0 11-18 0 9 9 0 0118 0z" />
                </svg>
                <span>{deleteError()}</span>
              </div>
            )}

            {deleteSuccess() && (
              <div class="alert alert-success">
                <svg xmlns="http://www.w3.org/2000/svg" class="stroke-current shrink-0 h-6 w-6" fill="none" viewBox="0 0 24 24">
                  <path stroke-linecap="round" stroke-linejoin="round" stroke-width="2" d="M9 12l2 2 4-4m6 2a9 9 0 11-18 0 9 9 0 0118 0z" />
                </svg>
                <span>{deleteSuccess()}</span>
              </div>
            )}
          </div>

          <div class="modal-action">
            <label for="delete-modal-toggle" class="btn btn-ghost" onClick={() => {
              setDeleteError("");
              setDeleteSuccess("");
            }}>
              Cancel
            </label>
            <button 
              class="btn btn-error" 
              onClick={handleDeleteConfirm}
              disabled={isDeleting()}
            >
              {isDeleting() ? (
                <>
                  <span class="loading loading-spinner loading-sm mr-2"></span>
                  Deleting...
                </>
              ) : (
                "Confirm Delete"
              )}
            </button>
          </div>
        </div>
        <label class="modal-backdrop" for="delete-modal-toggle">Close</label>
      </div>
    </div>
  );
}
