import { createSignal, createEffect, Show, For } from "solid-js";
import {
  HiOutlineInbox,
  HiOutlinePaperAirplane,
  HiOutlineDocument,
  HiOutlineStar,
  HiOutlineArchiveBox,
  HiOutlineTrash,
  HiOutlineMagnifyingGlass,
  HiOutlinePaperClip,
  HiOutlineFlag,
  HiOutlineEnvelope,
  HiOutlineEnvelopeOpen,
  HiOutlineFunnel,
} from "solid-icons/hi";
import type { Email, ListEmailsResponse } from "../api-types/types";
import { getApiUrl } from "../config/api";

export default function Emails() {
  const [emails, setEmails] = createSignal<Email[]>([]);
  const [loading, setLoading] = createSignal(true);
  const [error, setError] = createSignal<string | null>(null);
  const [selectedFolder, setSelectedFolder] = createSignal<string | null>(null);
  const [searchQuery, setSearchQuery] = createSignal("");
  const [selectedEmail, setSelectedEmail] = createSignal<Email | null>(null);

  // Mock folder data (since we don't have a folders API yet)
  const folders = [
    { name: "Inbox", icon: HiOutlineInbox, count: 42, folder: null },
    { name: "Sent", icon: HiOutlinePaperAirplane, count: 156, folder: "Sent" },
    { name: "Drafts", icon: HiOutlineDocument, count: 3, folder: "Drafts" },
    { name: "Starred", icon: HiOutlineStar, count: 12, folder: "Starred" },
    { name: "Archive", icon: HiOutlineArchiveBox, count: 892, folder: "Archive" },
    { name: "Trash", icon: HiOutlineTrash, count: 8, folder: "Trash" },
  ];

  // Fetch emails from API
  const fetchEmails = async () => {
    setLoading(true);
    setError(null);
    try {
      const params = new URLSearchParams();
      if (selectedFolder()) params.append("folder", selectedFolder()!);
      params.append("limit", "50");
      params.append("offset", "0");

      const url = getApiUrl(`/api/emails?${params.toString()}`);
      const response = await fetch(url);

      if (!response.ok) {
        throw new Error(`HTTP error! status: ${response.status}`);
      }

      const data: ListEmailsResponse = await response.json();
      setEmails(data.emails);
    } catch (err) {
      setError(err instanceof Error ? err.message : "Failed to fetch emails");
      console.error("Error fetching emails:", err);
    } finally {
      setLoading(false);
    }
  };

  // Fetch emails on mount and when folder changes
  createEffect(() => {
    selectedFolder(); // Track dependency
    fetchEmails();
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

  // Calculate inbox stats (mocked for now)
  const unreadCount = () => emails().filter((e) => !e.is_read).length;
  const flaggedCount = () => emails().filter((e) => e.is_flagged).length;
  const withAttachmentsCount = () => emails().filter((e) => e.has_attachments).length;

  return (
    <div class="flex h-screen bg-base-200">
      {/* Left Sidebar - Folders */}
      <div class="w-64 bg-base-100 border-r border-base-300 flex flex-col">
        {/* Compose Button */}
        <div class="p-4">
          <button class="btn btn-primary w-full gap-2">
            <HiOutlinePaperAirplane class="w-5 h-5" />
            Compose
          </button>
        </div>

        {/* Folders List */}
        <div class="flex-1 overflow-y-auto">
          <ul class="menu px-2">
            <For each={folders}>
              {(folder) => (
                <li>
                  <a
                    class="flex items-center justify-between"
                    classList={{
                      active: selectedFolder() === folder.folder,
                    }}
                    onClick={() => setSelectedFolder(folder.folder)}
                  >
                    <div class="flex items-center gap-3">
                      <folder.icon class="w-5 h-5" />
                      <span>{folder.name}</span>
                    </div>
                    <span class="badge badge-sm badge-ghost">{folder.count}</span>
                  </a>
                </li>
              )}
            </For>
          </ul>
        </div>

        {/* Storage Info (Mock) */}
        <div class="p-4 border-t border-base-300">
          <div class="text-xs text-base-content/60 mb-2">
            Storage: 2.3 GB of 15 GB used
          </div>
          <progress class="progress progress-primary w-full h-2" value="15" max="100"></progress>
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
              <div class="stat-value text-2xl text-primary">{unreadCount()}</div>
            </div>
            <div class="stat bg-base-200 rounded-lg p-4 flex-1">
              <div class="stat-title text-xs">Flagged</div>
              <div class="stat-value text-2xl text-warning">{flaggedCount()}</div>
            </div>
            <div class="stat bg-base-200 rounded-lg p-4 flex-1">
              <div class="stat-title text-xs">With Attachments</div>
              <div class="stat-value text-2xl text-accent">{withAttachmentsCount()}</div>
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
                      Your {selectedFolder() || "inbox"} is empty
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

                          {/* Labels */}
                          <Show when={email.labels.length > 0}>
                            <div class="flex gap-1 mt-2 flex-wrap">
                              <For each={email.labels.slice(0, 3)}>
                                {(label) => (
                                  <span class="badge badge-xs badge-outline">{label}</span>
                                )}
                              </For>
                            </div>
                          </Show>
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
            Showing {emails().length} emails
          </div>
          <div class="join">
            <button class="join-item btn btn-sm">«</button>
            <button class="join-item btn btn-sm">Page 1</button>
            <button class="join-item btn btn-sm">»</button>
          </div>
        </div>
      </div>
    </div>
  );
}
