import { createSignal, createEffect, Show, For, onMount } from "solid-js";
import { useParams, useNavigate, A } from "@solidjs/router";
import {
  HiOutlinePaperAirplane,
  HiOutlineMagnifyingGlass,
  HiOutlinePaperClip,
  HiOutlineFlag,
  HiOutlineEnvelope,
  HiOutlineEnvelopeOpen,
  HiOutlineFunnel,
  HiOutlineInbox,
  HiOutlineTag,
  HiOutlineCog6Tooth,
} from "solid-icons/hi";
import type {
  Email,
  ListEmailsResponse,
  CredentialMetadata,
  EmailFolder,
  EmailLabel,
} from "../api-types/types";
import {
  fetchCredentials,
  fetchFolders,
  fetchLabels,
  fetchEmailsByFolder,
  fetchEmailsByLabel,
  fetchEmailsByAccount,
} from "../api/emails";
import FolderIcon from "../components/emails/FolderIcon";

export default function Emails() {
  // Account & navigation state
  const [accounts, setAccounts] = createSignal<CredentialMetadata[]>([]);
  const [folders, setFolders] = createSignal<EmailFolder[]>([]);
  const [labels, setLabels] = createSignal<EmailLabel[]>([]);

  // Email list state
  const [emails, setEmails] = createSignal<Email[]>([]);
  const [loading, setLoading] = createSignal(true);
  const [error, setError] = createSignal<string | null>(null);
  const [totalCount, setTotalCount] = createSignal<bigint>(0n);
  const [hasMore, setHasMore] = createSignal(false);

  // UI state
  const [searchQuery, setSearchQuery] = createSignal("");
  const [selectedEmail, setSelectedEmail] = createSignal<Email | null>(null);

  // Router hooks
  const params = useParams<{
    accountId?: string;
    folderId?: string;
    labelId?: string;
  }>();
  const navigate = useNavigate();

  // Load accounts on mount
  onMount(async () => {
    try {
      const creds = await fetchCredentials();
      setAccounts(creds);

      // If no account in URL, navigate to first account
      if (!params.accountId && creds.length > 0) {
        navigate(`/emails/account/${creds[0].id.toString()}`, { replace: true });
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to load accounts");
    }
  });

  // Load folders/labels when account changes
  createEffect(async () => {
    const accountId = params.accountId;
    if (!accountId) return;

    try {
      const credId = BigInt(accountId);
      const [foldersData, labelsData] = await Promise.all([
        fetchFolders(credId),
        fetchLabels(credId),
      ]);

      setFolders(foldersData);
      setLabels(labelsData);

      // Auto-navigate to INBOX if no folder/label selected
      if (!params.folderId && !params.labelId) {
        const inbox = foldersData.find(
          f => f.name.toLowerCase() === 'inbox'
        );
        if (inbox) {
          navigate(
            `/emails/account/${accountId}/folder/${inbox.id}`,
            { replace: true }
          );
        }
      }
    } catch (err) {
      console.error("Error loading folders/labels:", err);
    }
  });

  // Load emails when folder/label/account changes
  createEffect(async () => {
    const { accountId, folderId, labelId } = params;

    if (!accountId) return;

    setLoading(true);
    setError(null);

    try {
      let response: ListEmailsResponse;

      if (folderId) {
        response = await fetchEmailsByFolder(BigInt(folderId), 50, 0);
      } else if (labelId) {
        response = await fetchEmailsByLabel(BigInt(labelId), 50, 0);
      } else {
        response = await fetchEmailsByAccount(BigInt(accountId), 50, 0);
      }

      setEmails(response.emails);
      setTotalCount(response.total_count);
      setHasMore(response.has_more);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to fetch emails");
    } finally {
      setLoading(false);
    }
  });

  // Format date for display
  const formatDate = (timestamp: bigint): string => {
    const date = new Date(Number(timestamp));
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffMins = Math.floor(diffMs / 60000);
    const diffHours = Math.floor(diffMs / 3600000);
    const diffDays = Math.floor(diffMs / 86400000);

    if (diffMins < 1) return "Just now";
    if (diffMins < 60) return `${diffMins}m ago`;
    if (diffHours < 24) return `${diffHours}h ago`;
    if (diffDays === 0) return "Today";
    if (diffDays === 1) return "Yesterday";
    if (diffDays < 7) return `${diffDays}d ago`;

    return date.toLocaleDateString("en-US", { month: "short", day: "numeric" });
  };

  // Get email preview text
  const getPreviewText = (email: Email): string => {
    const text = email.body_text || email.body_html || "";
    return text.substring(0, 120).replace(/<[^>]*>/g, "");
  };

  // Calculate inbox stats
  const unreadCount = () => emails().filter((e) => !e.is_read).length;
  const flaggedCount = () => emails().filter((e) => e.is_flagged).length;
  const withAttachmentsCount = () =>
    emails().filter((e) => e.has_attachments).length;

  return (
    <div class="flex h-screen bg-base-200">
      {/* No Accounts Empty State */}
      <Show when={accounts().length === 0 && !loading()}>
        <div class="flex-1 flex flex-col items-center justify-center text-center p-8">
          <HiOutlineEnvelope class="w-24 h-24 text-base-content/30 mb-4" />
          <h3 class="text-xl font-semibold mb-2">No email accounts configured</h3>
          <p class="text-base-content/60 mb-6">
            Add your first email account to start viewing messages
          </p>
          <A href="/settings/accounts" class="btn btn-primary gap-2">
            <HiOutlineCog6Tooth class="w-5 h-5" />
            Go to Account Settings
          </A>
        </div>
      </Show>

      {/* Left Sidebar - Accounts, Folders, Labels */}
      <Show when={accounts().length > 0}>
        <div class="w-64 bg-base-100 border-r border-base-300 flex flex-col">
          {/* Account Selector & Compose Button */}
          <div class="p-4 border-b border-base-300">
            <Show
              when={accounts().length > 0}
              fallback={
                <div class="text-center text-sm text-base-content/60">
                  No accounts found
                </div>
              }
            >
              <select
                class="select select-bordered select-sm w-full mb-3"
                value={params.accountId || ""}
                onChange={(e) => {
                  const accountId = e.currentTarget.value;
                  if (accountId) {
                    navigate(`/emails/account/${accountId}`);
                  }
                }}
              >
                <For each={accounts()}>
                  {(account) => (
                    <option value={account.id.toString()}>
                      {account.identifier}
                    </option>
                  )}
                </For>
              </select>
            </Show>

            <button class="btn btn-primary w-full gap-2">
              <HiOutlinePaperAirplane class="w-5 h-5" />
              Compose
            </button>
          </div>

          {/* Folders & Labels List */}
          <div class="flex-1 overflow-y-auto">
            <Show
              when={params.accountId}
              fallback={
                <div class="p-4 text-center text-sm text-base-content/60">
                  Select an account to view folders
                </div>
              }
            >
              <ul class="menu px-2">
                {/* Folders Section */}
                <Show when={folders().length > 0}>
                  <li class="menu-title">Folders</li>
                  <For each={folders()}>
                    {(folder) => (
                      <li class="w-full">
                        <A
                          href={`/emails/account/${params.accountId}/folder/${folder.id}`}
                          class="flex items-center justify-between w-full"
                          activeClass="active"
                        >
                          <div class="flex items-center gap-3 flex-1 min-w-0">
                            <FolderIcon folderType={folder.folder_type} />
                            <span class="truncate">
                              {folder.display_name || folder.name}
                            </span>
                          </div>
                          <span class="badge badge-sm badge-ghost ml-2 flex-shrink-0">
                            <Show
                              when={folder.unread_messages > 0}
                              fallback={folder.total_messages}
                            >
                              {folder.unread_messages}/{folder.total_messages}
                            </Show>
                          </span>
                        </A>
                      </li>
                    )}
                  </For>
                </Show>

                {/* Labels Section */}
                <Show when={labels().length > 0}>
                  <li class="menu-title mt-4">Labels</li>
                  <For each={labels()}>
                    {(label) => (
                      <li class="w-full">
                        <A
                          href={`/emails/account/${params.accountId}/label/${label.id}`}
                          class="flex items-center justify-between w-full"
                          activeClass="active"
                        >
                          <div class="flex items-center gap-3 flex-1 min-w-0">
                            <HiOutlineTag class="w-5 h-5" />
                            <span class="truncate">
                              {label.display_name || label.name}
                            </span>
                          </div>
                          <span class="badge badge-sm badge-ghost ml-2 flex-shrink-0">
                            {label.message_count}
                          </span>
                        </A>
                      </li>
                    )}
                  </For>
                </Show>

                {/* Empty State */}
                <Show when={folders().length === 0 && labels().length === 0}>
                  <li class="text-center text-sm text-base-content/60 py-4">
                    No folders or labels found
                  </li>
                </Show>
              </ul>
            </Show>
          </div>
        </div>

        {/* Main Content Area */}
        <div class="flex-1 flex flex-col">
          {/* Top Bar - Stats & Search */}
          <div class="bg-base-100 border-b border-base-300 p-4">
            {/* Stats */}
            <div class="flex gap-4 mb-4">
              <div class="stat bg-base-200 rounded-lg p-4 flex-1">
                <div class="stat-title text-xs">Unread</div>
                <div class="stat-value text-2xl text-primary">
                  {unreadCount()}
                </div>
              </div>
              <div class="stat bg-base-200 rounded-lg p-4 flex-1">
                <div class="stat-title text-xs">Flagged</div>
                <div class="stat-value text-2xl text-warning">
                  {flaggedCount()}
                </div>
              </div>
              <div class="stat bg-base-200 rounded-lg p-4 flex-1">
                <div class="stat-title text-xs">With Attachments</div>
                <div class="stat-value text-2xl text-accent">
                  {withAttachmentsCount()}
                </div>
              </div>
            </div>

            {/* Search Bar */}
            <div class="flex gap-2">
              <div class="form-control flex-1">
                <div class="input-group">
                  <span class="bg-base-200">
                    <HiOutlineMagnifyingGlass class="w-5 h-5" />
                  </span>
                  <input
                    type="text"
                    placeholder="Search emails..."
                    class="input input-bordered w-full"
                    value={searchQuery()}
                    onInput={(e) => setSearchQuery(e.currentTarget.value)}
                  />
                </div>
              </div>
              <button class="btn btn-square btn-outline">
                <HiOutlineFunnel class="w-5 h-5" />
              </button>
            </div>
          </div>

          {/* Email List */}
          <div class="flex-1 overflow-y-auto">
            <Show
              when={!loading()}
              fallback={
                <div class="flex items-center justify-center h-full">
                  <span class="loading loading-spinner loading-lg text-primary"></span>
                </div>
              }
            >
              <Show
                when={!error()}
                fallback={
                  <div class="flex items-center justify-center h-full">
                    <div class="alert alert-error max-w-md">
                      <span>{error()}</span>
                    </div>
                  </div>
                }
              >
                <Show
                  when={emails().length > 0}
                  fallback={
                    <div class="flex flex-col items-center justify-center h-full text-center p-8">
                      <HiOutlineInbox class="w-24 h-24 text-base-content/30 mb-4" />
                      <h3 class="text-xl font-semibold mb-2">No emails found</h3>
                      <p class="text-base-content/60">
                        {(() => {
                          if (params.folderId) {
                            const folder = folders().find(f => f.id.toString() === params.folderId);
                            return `Your ${folder?.name || "folder"} is empty`;
                          }
                          if (params.labelId) {
                            const label = labels().find(l => l.id.toString() === params.labelId);
                            return `No emails with label "${label?.name || "this label"}"`;
                          }
                          return "No emails found";
                        })()}
                      </p>
                    </div>
                  }
                >
                  <div class="divide-y divide-base-300">
                    <For each={emails()}>
                      {(email) => (
                        <div
                          class="flex items-start gap-4 p-4 hover:bg-base-200 cursor-pointer transition-colors"
                          classList={{
                            "bg-base-100": email.is_read,
                            "bg-base-200/50 font-medium": !email.is_read,
                          }}
                          onClick={() => setSelectedEmail(email)}
                        >
                          {/* Left: Checkbox & Icons */}
                          <div class="flex items-center gap-3 pt-1">
                            <input
                              type="checkbox"
                              class="checkbox checkbox-sm"
                              onClick={(e) => e.stopPropagation()}
                            />
                            <Show when={!email.is_read}>
                              <HiOutlineEnvelope class="w-5 h-5 text-primary" />
                            </Show>
                            <Show when={email.is_read}>
                              <HiOutlineEnvelopeOpen class="w-5 h-5 text-base-content/40" />
                            </Show>
                            <Show when={email.is_flagged}>
                              <HiOutlineFlag class="w-5 h-5 text-warning fill-current" />
                            </Show>
                          </div>

                          {/* Middle: Email Content */}
                          <div class="flex-1 min-w-0">
                            {/* From & Subject */}
                            <div class="flex items-baseline gap-2 mb-1">
                              <span
                                class="text-sm truncate"
                                classList={{
                                  "font-semibold": !email.is_read,
                                  "text-base-content/70": email.is_read,
                                }}
                              >
                                {email.from_name || email.from_address}
                              </span>
                              <Show when={email.has_attachments}>
                                <HiOutlinePaperClip class="w-4 h-4 text-base-content/60 flex-shrink-0" />
                              </Show>
                            </div>

                            {/* Subject */}
                            <div
                              class="text-sm mb-1 truncate"
                              classList={{
                                "font-medium": !email.is_read,
                                "text-base-content/60": email.is_read,
                              }}
                            >
                              {email.subject || "(No subject)"}
                            </div>

                            {/* Preview */}
                            <div class="text-xs text-base-content/50 truncate">
                              {getPreviewText(email)}
                            </div>
                          </div>

                          {/* Right: Date & Meta */}
                          <div class="flex flex-col items-end gap-2 pt-1">
                            <span class="text-xs text-base-content/60 whitespace-nowrap">
                              {formatDate(email.date_received)}
                            </span>
                            <Show when={email.attachment_count > 0}>
                              <div class="badge badge-sm badge-ghost gap-1">
                                <HiOutlinePaperClip class="w-3 h-3" />
                                {email.attachment_count}
                              </div>
                            </Show>
                          </div>
                        </div>
                      )}
                    </For>
                  </div>
                </Show>
              </Show>
            </Show>
          </div>

          {/* Bottom Pagination Bar */}
          <div class="bg-base-100 border-t border-base-300 p-3 flex items-center justify-between">
            <div class="text-sm text-base-content/60">
              Showing {emails().length} of {totalCount().toString()} emails
            </div>
            <div class="join">
              <button class="join-item btn btn-sm">«</button>
              <button class="join-item btn btn-sm">Page 1</button>
              <button class="join-item btn btn-sm">»</button>
            </div>
          </div>
        </div>
      </Show>
    </div>
  );
}
