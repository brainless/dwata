import { createSignal, onCleanup } from "solid-js";
import { getApiUrl } from "../config/api";

interface AccountProgress {
  credential_id: number;
  identifier: string;
  status: string;
  total_emails: number;
  emails_processed: number;
  emails_failed: number;
  current_email_id: number | null;
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
