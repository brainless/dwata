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
    "success"
  );

  // Credentials list state
  const [credentials, setCredentials] = createSignal<CredentialMetadata[]>([]);
  const [isLoadingList, setIsLoadingList] = createSignal(true);

  // Fetch credentials on mount
  onMount(async () => {
    await fetchCredentials();
  });

  const fetchCredentials = async () => {
    setIsLoadingList(true);
    try {
      const response = await fetch("http://localhost:8080/api/credentials");
      if (response.ok) {
        const data: CredentialListResponse = await response.json();
        setCredentials(data.credentials);
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
        `http://localhost:8080/api/credentials/${id}?hard=true`,
        {
          method: "DELETE",
        }
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

      const response = await fetch("http://localhost:8080/api/credentials", {
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

  const credentialTypeConfig = {
    imap: { label: "IMAP", icon: HiOutlineEnvelope, badgeClass: "badge-primary" },
    smtp: { label: "SMTP", icon: HiOutlineEnvelope, badgeClass: "badge-secondary" },
    oauth: { label: "OAuth", icon: HiOutlineEnvelope, badgeClass: "badge-accent" },
    apikey: { label: "API Key", icon: HiOutlineEnvelope, badgeClass: "badge-info" },
    database: { label: "Database", icon: HiOutlineEnvelope, badgeClass: "badge-warning" },
    custom: { label: "Custom", icon: HiOutlineEnvelope, badgeClass: "badge-ghost" },
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
                        const typeConfig =
                          credentialTypeConfig[
                            credential.credential_type as keyof typeof credentialTypeConfig
                          ];
                        const TypeIcon = typeConfig.icon;

                        return (
                          <tr>
                            <td>
                              <div>
                                <div class="font-bold">{credential.identifier}</div>
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
                                      credential.identifier
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

      {/* Add New Account Form */}
      <div class="card bg-base-100 shadow-xl">
        <div class="card-body">
          <h2 class="card-title">Add New Email Account</h2>
          <p class="text-sm text-base-content/70 mb-4">
            Add your IMAP email accounts to enable email ingestion and monitoring.
          </p>

        {/* IMAP Email Form */}
        <form onSubmit={handleSubmit} class="space-y-4">
          <fieldset class="border border-base-300 rounded-lg p-4">
            <legend class="text-lg font-semibold px-2">IMAP Email Account</legend>

            {/* Account Information */}
            <div class="space-y-4 mt-4">
              <div class="form-control w-full">
                <label class="label">
                  <span class="label-text">Account Name *</span>
                  <span class="label-text-alt text-xs">Unique identifier</span>
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
                    <span class="label-text font-medium">Validate SSL Certificates</span>
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
