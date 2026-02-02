import { createSignal, onMount, Show, For } from "solid-js";
import {
  HiOutlineEnvelope,
  HiOutlineTrash,
  HiOutlineEye,
  HiOutlineCheckCircle,
  HiOutlineXCircle,
} from "solid-icons/hi";
import type {
  CreateImapCredentialRequest,
  ImapAccountSettings,
  ImapAuthMethod,
  CredentialMetadata,
  CredentialListResponse,
} from "../../api-types/types";
import { getApiUrl } from "../../config/api";

export default function SettingsAccounts() {
  // Form state
  const [identifier, setIdentifier] = createSignal("");
  const [username, setUsername] = createSignal("");
  const [password, setPassword] = createSignal("");
  const [host, setHost] = createSignal("");
  const [port, setPort] = createSignal("993");
  const [useTls, setUseTls] = createSignal(true);
  const [authMethod, setAuthMethod] = createSignal<ImapAuthMethod>("plain");
  const [defaultMailbox, setDefaultMailbox] = createSignal("INBOX");
  const [connectionTimeout, setConnectionTimeout] = createSignal("30");
  const [validateCerts, setValidateCerts] = createSignal(true);
  const [notes, setNotes] = createSignal("");

  // UI state
  const [isLoading, setIsLoading] = createSignal(false);
  const [message, setMessage] = createSignal("");
  const [messageType, setMessageType] = createSignal<"success" | "error">(
    "success",
  );

  // Gmail OAuth state
  const [isGmailLoading, setIsGmailLoading] = createSignal(false);
  const [gmailMessage, setGmailMessage] = createSignal("");
  const [gmailMessageType, setGmailMessageType] = createSignal<
    "success" | "error"
  >("success");

  // Credentials list state
  const [credentials, setCredentials] = createSignal<CredentialMetadata[]>([]);
  const [isLoadingList, setIsLoadingList] = createSignal(false);

  // Fetch credentials on mount
  onMount(async () => {
    await fetchCredentials();
  });

  const fetchCredentials = async () => {
    setIsLoadingList(true);
    try {
      const response = await fetch(getApiUrl("/api/credentials"));
      if (response.ok) {
        const data: CredentialListResponse = await response.json();
        console.log("Fetched credentials:", data.credentials);
        console.log("Credentials count:", data.credentials.length);
        setCredentials(data.credentials);
      } else {
        console.error("Failed to fetch credentials, status:", response.status);
      }
    } catch (error) {
      console.error("Failed to fetch credentials:", error);
    } finally {
      setIsLoadingList(false);
    }
  };

  const deleteCredential = async (id: string, identifier: string) => {
    if (!confirm(`Are you sure you want to delete "${identifier}"?`)) {
      return;
    }

    try {
      const response = await fetch(
        getApiUrl(`/api/credentials/${id}?hard=true`),
        {
          method: "DELETE",
        },
      );

      if (response.ok) {
        setMessageType("success");
        setMessage(`Credential "${identifier}" deleted successfully!`);
        await fetchCredentials(); // Refresh list
      } else {
        setMessageType("error");
        setMessage(`Failed to delete credential "${identifier}".`);
      }
    } catch (error) {
      console.error("Failed to delete credential:", error);
      setMessageType("error");
      setMessage("Failed to delete credential. Please try again.");
    }
  };

  const handleGmailSignIn = async () => {
    setIsGmailLoading(true);
    setGmailMessage("");

    try {
      // Call initiate endpoint to get authorization URL
      const response = await fetch(
        getApiUrl("/api/credentials/gmail/initiate"),
        {
          method: "POST",
        },
      );

      if (response.ok) {
        const data = await response.json();
        const { authorization_url } = data;

        // Open authorization URL in new window
        const authWindow = window.open(
          authorization_url,
          "_blank",
          "width=600,height=700,menubar=no,toolbar=no,location=no,status=no",
        );

        if (authWindow) {
          setGmailMessageType("success");
          setGmailMessage(
            "Please complete the sign-in in the popup window. It will close automatically when done.",
          );

          // Poll for new credentials after OAuth flow completes
          const pollInterval = setInterval(async () => {
            if (authWindow.closed) {
              clearInterval(pollInterval);
              setGmailMessage("");
              await fetchCredentials();
            }
          }, 1000);

          // Stop polling after 5 minutes
          setTimeout(() => {
            clearInterval(pollInterval);
            if (!authWindow.closed) {
              setGmailMessageType("error");
              setGmailMessage("Sign-in timed out. Please try again.");
            }
          }, 300000);
        } else {
          setGmailMessageType("error");
          setGmailMessage(
            "Failed to open sign-in window. Please allow popups for this site.",
          );
        }
      } else {
        const error = await response.json();
        setGmailMessageType("error");
        setGmailMessage(error.error || "Failed to initiate Gmail sign-in.");
      }
    } catch (error) {
      console.error("Failed to initiate Gmail OAuth:", error);
      setGmailMessageType("error");
      setGmailMessage("Failed to connect to server. Please try again.");
    } finally {
      setIsGmailLoading(false);
    }
  };

  const handleSubmit = async (e: Event) => {
    e.preventDefault();
    setIsLoading(true);
    setMessage("");

    try {
      const settings: ImapAccountSettings = {
        host: host(),
        port: parseInt(port()),
        use_tls: useTls(),
        auth_method: authMethod(),
        default_mailbox: defaultMailbox(),
        connection_timeout_secs: parseInt(connectionTimeout()),
        validate_certs: validateCerts(),
      };

      const request: CreateImapCredentialRequest = {
        identifier: identifier(),
        username: username(),
        password: password(),
        settings,
        notes: notes() || null,
      };

      // Convert CreateImapCredentialRequest to generic CreateCredentialRequest format
      const genericRequest = {
        credential_type: "imap",
        identifier: request.identifier,
        username: request.username,
        password: request.password,
        service_name: settings.host,
        port: settings.port,
        use_tls: settings.use_tls,
        notes: request.notes,
        extra_metadata: JSON.stringify({
          auth_method: settings.auth_method,
          default_mailbox: settings.default_mailbox,
          connection_timeout_secs: settings.connection_timeout_secs,
          validate_certs: settings.validate_certs,
        }),
      };

      const response = await fetch(getApiUrl("/api/credentials"), {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify(genericRequest),
      });

      if (response.ok) {
        setMessageType("success");
        setMessage("IMAP account added successfully!");
        // Reset form
        setIdentifier("");
        setUsername("");
        setPassword("");
        setHost("");
        setPort("993");
        setUseTls(true);
        setAuthMethod("plain");
        setDefaultMailbox("INBOX");
        setConnectionTimeout("30");
        setValidateCerts(true);
        setNotes("");
        // Refresh credentials list
        await fetchCredentials();
      } else {
        const error = await response.json();
        setMessageType("error");
        setMessage(error.error || "Failed to add IMAP account.");
      }
    } catch (error) {
      console.error("Failed to add IMAP account:", error);
      setMessageType("error");
      setMessage("Failed to add IMAP account. Please try again.");
    } finally {
      setIsLoading(false);
    }
  };

  const credentialTypeConfig: Record<string, { label: string, icon: any, badgeClass: string }> = {
    imap: {
      label: "IMAP",
      icon: HiOutlineEnvelope,
      badgeClass: "badge-primary",
    },
    smtp: {
      label: "SMTP",
      icon: HiOutlineEnvelope,
      badgeClass: "badge-secondary",
    },
    oauth: {
      label: "OAuth",
      icon: HiOutlineEnvelope,
      badgeClass: "badge-accent",
    },
    apikey: {
      label: "API Key",
      icon: HiOutlineEnvelope,
      badgeClass: "badge-info",
    },
    database: {
      label: "Database",
      icon: HiOutlineEnvelope,
      badgeClass: "badge-warning",
    },
    localfile: {
      label: "Local File",
      icon: HiOutlineEnvelope,
      badgeClass: "badge-ghost",
    },
    custom: {
      label: "Custom",
      icon: HiOutlineEnvelope,
      badgeClass: "badge-ghost",
    },
  };

  return (
    <div class="space-y-6">
      {/* Existing Credentials List */}
      <div class="card bg-base-100 shadow-xl">
        <div class="card-body">
          <h2 class="card-title">Your Email Accounts</h2>
          <p class="text-sm text-base-content/70 mb-4">
            Manage your configured email accounts
          </p>

          <Show
            when={!isLoadingList()}
            fallback={
              <div class="flex justify-center py-8">
                <span class="loading loading-spinner loading-lg"></span>
              </div>
            }
          >
            <Show
              when={credentials().length > 0}
              fallback={
                <div class="text-center py-8 text-base-content/60">
                  <HiOutlineEnvelope class="w-12 h-12 mx-auto mb-2 opacity-30" />
                  <p>No email accounts configured yet.</p>
                  <p class="text-sm">Add your first account below.</p>
                </div>
              }
            >
              <div class="overflow-x-auto">
                <table class="table table-zebra">
                  <thead>
                    <tr>
                      <th>Account</th>
                      <th>Type</th>
                      <th>Server</th>
                      <th>Status</th>
                      <th></th>
                    </tr>
                  </thead>
                  <tbody>
                    <For each={credentials()}>
                      {(credential) => {
                        console.log("Rendering credential:", credential);
                        console.log("Credential type:", credential.credential_type);
                        const typeConfig =
                          credentialTypeConfig[
                            credential.credential_type as keyof typeof credentialTypeConfig
                          ];
                        console.log("Type config:", typeConfig);
                        const TypeIcon = typeConfig.icon;

                        return (
                          <tr>
                            <td>
                              <div>
                                <div class="font-bold">
                                  {credential.identifier}
                                </div>
                                <div class="text-sm opacity-60">
                                  {credential.username}
                                </div>
                              </div>
                            </td>
                            <td>
                              <span
                                class={`badge badge-sm ${typeConfig.badgeClass} gap-1`}
                              >
                                <TypeIcon class="w-3 h-3" />
                                {typeConfig.label}
                              </span>
                            </td>
                            <td>
                              <div class="text-sm">
                                {credential.service_name && (
                                  <div>{credential.service_name}</div>
                                )}
                                {credential.port && (
                                  <div class="text-xs opacity-60">
                                    Port: {credential.port}
                                  </div>
                                )}
                              </div>
                            </td>
                            <td>
                              <span
                                class={`badge badge-sm gap-1 ${
                                  credential.is_active
                                    ? "badge-success"
                                    : "badge-ghost"
                                }`}
                              >
                                {credential.is_active ? (
                                  <>
                                    <HiOutlineCheckCircle class="w-3 h-3" />
                                    Active
                                  </>
                                ) : (
                                  <>
                                    <HiOutlineXCircle class="w-3 h-3" />
                                    Inactive
                                  </>
                                )}
                              </span>
                            </td>
                            <td>
                              <div class="flex gap-2">
                                <button
                                  class="btn btn-ghost btn-sm btn-circle"
                                  title="View details"
                                >
                                  <HiOutlineEye class="w-4 h-4" />
                                </button>
                                <button
                                  class="btn btn-ghost btn-sm btn-circle text-error hover:bg-error hover:text-error-content"
                                  title="Delete account"
                                  onClick={() =>
                                    deleteCredential(
                                      credential.id,
                                      credential.identifier,
                                    )
                                  }
                                >
                                  <HiOutlineTrash class="w-4 h-4" />
                                </button>
                              </div>
                            </td>
                          </tr>
                        );
                      }}
                    </For>
                  </tbody>
                </table>
              </div>
            </Show>
          </Show>
        </div>
      </div>

      {/* Add Gmail Account */}
      <div class="card bg-base-100 shadow-xl">
        <div class="card-body">
          <h2 class="card-title">Add Gmail Account</h2>
          <p class="text-sm text-base-content/70 mb-4">
            Connect your Gmail account using secure OAuth2 authentication. No
            password required!
          </p>

          <div class="flex flex-col items-center justify-center py-8 space-y-4">
            <div class="text-center max-w-md">
              <p class="text-sm mb-6">
                Click the button below to sign in with your Google account.
                You'll be redirected to Google's secure login page to authorize
                dwata to access your Gmail.
              </p>
            </div>

            <button
              type="button"
              class="btn btn-lg bg-white hover:bg-gray-50 text-gray-800 border border-gray-300 shadow-md gap-3"
              onClick={handleGmailSignIn}
              disabled={isGmailLoading()}
            >
              {isGmailLoading() ? (
                <>
                  <span class="loading loading-spinner loading-md"></span>
                  Connecting...
                </>
              ) : (
                <>
                  <svg class="w-6 h-6" viewBox="0 0 24 24">
                    <path
                      fill="#4285F4"
                      d="M22.56 12.25c0-.78-.07-1.53-.2-2.25H12v4.26h5.92c-.26 1.37-1.04 2.53-2.21 3.31v2.77h3.57c2.08-1.92 3.28-4.74 3.28-8.09z"
                    />
                    <path
                      fill="#34A853"
                      d="M12 23c2.97 0 5.46-.98 7.28-2.66l-3.57-2.77c-.98.66-2.23 1.06-3.71 1.06-2.86 0-5.29-1.93-6.16-4.53H2.18v2.84C3.99 20.53 7.7 23 12 23z"
                    />
                    <path
                      fill="#FBBC05"
                      d="M5.84 14.09c-.22-.66-.35-1.36-.35-2.09s.13-1.43.35-2.09V7.07H2.18C1.43 8.55 1 10.22 1 12s.43 3.45 1.18 4.93l2.85-2.22.81-.62z"
                    />
                    <path
                      fill="#EA4335"
                      d="M12 5.38c1.62 0 3.06.56 4.21 1.64l3.15-3.15C17.45 2.09 14.97 1 12 1 7.7 1 3.99 3.47 2.18 7.07l3.66 2.84c.87-2.6 3.3-4.53 6.16-4.53z"
                    />
                  </svg>
                  Sign in with Google
                </>
              )}
            </button>

            {/* Gmail Message Display */}
            {gmailMessage() && (
              <div
                class={`alert ${gmailMessageType() === "success" ? "alert-info" : "alert-error"} max-w-md`}
              >
                <span class="text-sm">{gmailMessage()}</span>
              </div>
            )}

            <div class="text-xs text-base-content/60 max-w-md text-center mt-4">
              <p>
                ✓ Secure OAuth2 authentication
                <br />
                ✓ No password storage required
                <br />✓ Easily revoke access anytime from your Google Account
                settings
              </p>
            </div>
          </div>
        </div>
      </div>

      {/* Add IMAP Account Form */}
      <div class="card bg-base-100 shadow-xl">
        <div class="card-body">
          <h2 class="card-title">Add Other IMAP Account</h2>
          <p class="text-sm text-base-content/70 mb-4">
            Add IMAP email accounts from other providers (e.g., Outlook, Yahoo,
            custom servers).
          </p>

          {/* IMAP Email Form */}
          <form onSubmit={handleSubmit} class="space-y-4">
            <fieldset class="border border-base-300 rounded-lg p-4">
              <legend class="text-lg font-semibold px-2">
                IMAP Email Account
              </legend>

              {/* Account Information */}
              <div class="space-y-4 mt-4">
                <div class="form-control w-full">
                  <label class="label">
                    <span class="label-text">Account Name *</span>
                    <span class="label-text-alt text-xs">
                      Unique identifier
                    </span>
                  </label>
                  <input
                    type="text"
                    placeholder="e.g., work_email, personal_gmail"
                    class="input input-bordered w-full"
                    value={identifier()}
                    onInput={(e) => setIdentifier(e.target.value)}
                    required
                  />
                </div>

                <div class="form-control w-full">
                  <label class="label">
                    <span class="label-text">Email Address *</span>
                  </label>
                  <input
                    type="email"
                    placeholder="your.email@example.com"
                    class="input input-bordered w-full"
                    value={username()}
                    onInput={(e) => setUsername(e.target.value)}
                    required
                  />
                </div>

                <div class="form-control w-full">
                  <label class="label">
                    <span class="label-text">Password *</span>
                    <span class="label-text-alt text-xs">
                      Stored securely in OS keychain
                    </span>
                  </label>
                  <input
                    type="password"
                    placeholder="••••••••"
                    class="input input-bordered w-full"
                    value={password()}
                    onInput={(e) => setPassword(e.target.value)}
                    required
                  />
                </div>
              </div>

              {/* Server Settings */}
              <div class="divider">Server Settings</div>

              <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                <div class="form-control w-full">
                  <label class="label">
                    <span class="label-text">IMAP Server Host *</span>
                  </label>
                  <input
                    type="text"
                    placeholder="imap.gmail.com"
                    class="input input-bordered w-full"
                    value={host()}
                    onInput={(e) => setHost(e.target.value)}
                    required
                  />
                </div>

                <div class="form-control w-full">
                  <label class="label">
                    <span class="label-text">Port *</span>
                  </label>
                  <input
                    type="number"
                    placeholder="993"
                    class="input input-bordered w-full"
                    value={port()}
                    onInput={(e) => setPort(e.target.value)}
                    required
                  />
                </div>
              </div>

              <div class="grid grid-cols-1 md:grid-cols-2 gap-4 mt-4">
                <div class="form-control">
                  <label class="label cursor-pointer justify-start gap-4">
                    <input
                      type="checkbox"
                      class="checkbox checkbox-primary"
                      checked={useTls()}
                      onChange={(e) => setUseTls(e.target.checked)}
                    />
                    <div>
                      <span class="label-text font-medium">Use TLS/SSL</span>
                      <span class="label-text-alt block text-xs">
                        Recommended for secure connection
                      </span>
                    </div>
                  </label>
                </div>

                <div class="form-control">
                  <label class="label cursor-pointer justify-start gap-4">
                    <input
                      type="checkbox"
                      class="checkbox checkbox-primary"
                      checked={validateCerts()}
                      onChange={(e) => setValidateCerts(e.target.checked)}
                    />
                    <div>
                      <span class="label-text font-medium">
                        Validate SSL Certificates
                      </span>
                      <span class="label-text-alt block text-xs">
                        Should be enabled in production
                      </span>
                    </div>
                  </label>
                </div>
              </div>

              {/* Advanced Settings */}
              <div class="divider">Advanced Settings</div>

              <div class="grid grid-cols-1 md:grid-cols-3 gap-4">
                <div class="form-control w-full">
                  <label class="label">
                    <span class="label-text">Authentication Method</span>
                  </label>
                  <select
                    class="select select-bordered w-full"
                    value={authMethod()}
                    onChange={(e) =>
                      setAuthMethod(e.target.value as ImapAuthMethod)
                    }
                  >
                    <option value="plain">Plain</option>
                    <option value="oauth2">OAuth2</option>
                    <option value="xoauth2">XOAUTH2</option>
                  </select>
                </div>

                <div class="form-control w-full">
                  <label class="label">
                    <span class="label-text">Default Mailbox</span>
                  </label>
                  <input
                    type="text"
                    placeholder="INBOX"
                    class="input input-bordered w-full"
                    value={defaultMailbox()}
                    onInput={(e) => setDefaultMailbox(e.target.value)}
                  />
                </div>

                <div class="form-control w-full">
                  <label class="label">
                    <span class="label-text">Connection Timeout (sec)</span>
                  </label>
                  <input
                    type="number"
                    placeholder="30"
                    class="input input-bordered w-full"
                    value={connectionTimeout()}
                    onInput={(e) => setConnectionTimeout(e.target.value)}
                  />
                </div>
              </div>

              {/* Notes */}
              <div class="form-control w-full mt-4">
                <label class="label">
                  <span class="label-text">Notes (Optional)</span>
                </label>
                <textarea
                  class="textarea textarea-bordered w-full"
                  placeholder="Additional notes about this account..."
                  rows={2}
                  value={notes()}
                  onInput={(e) => setNotes(e.target.value)}
                />
              </div>
            </fieldset>

            {/* Message Display */}
            {message() && (
              <div
                class={`alert ${messageType() === "success" ? "alert-success" : "alert-error"}`}
              >
                <span>{message()}</span>
              </div>
            )}

            {/* CTA Button */}
            <div class="card-actions justify-end">
              <button
                type="submit"
                class="btn btn-primary btn-wide"
                disabled={isLoading()}
              >
                {isLoading() ? (
                  <>
                    <span class="loading loading-spinner loading-sm"></span>
                    Adding Account...
                  </>
                ) : (
                  "Add IMAP Account"
                )}
              </button>
            </div>
          </form>
        </div>
      </div>
    </div>
  );
}
