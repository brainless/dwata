import { For, Show, createEffect, createMemo, createSignal } from "solid-js";
import { A } from "@solidjs/router";
import type { EmailFolder } from "../../api-types/types";
import FolderIcon from "./FolderIcon";

type FolderTreeProps = {
  folders: EmailFolder[];
  accountId?: string;
  activeFolderId?: string;
  privacyEnabled?: boolean;
};

type FolderNode = {
  name: string;
  path: string;
  children: Map<string, FolderNode>;
  folder?: EmailFolder;
};

const buildTree = (folders: EmailFolder[]): FolderNode => {
  const root: FolderNode = {
    name: "",
    path: "",
    children: new Map(),
  };

  for (const folder of folders) {
    const rawName = folder.display_name || folder.name;
    const segments = rawName.split("/").filter(Boolean);
    let current = root;
    let currentPath = "";

    for (const segment of segments) {
      currentPath = currentPath ? `${currentPath}/${segment}` : segment;
      if (!current.children.has(segment)) {
        current.children.set(segment, {
          name: segment,
          path: currentPath,
          children: new Map(),
        });
      }
      current = current.children.get(segment)!;
    }

    current.folder = folder;
  }

  return root;
};

const sortNodes = (nodes: FolderNode[]) =>
  nodes.sort((a, b) => a.name.localeCompare(b.name));

export default function FolderTree(props: FolderTreeProps) {
  const [expanded, setExpanded] = createSignal<Set<string>>(new Set());

  const tree = createMemo(() => buildTree(props.folders || []));

  const activeFolder = createMemo(() => {
    const activeId = props.activeFolderId;
    if (!activeId) return null;
    return props.folders.find((f) => f.id.toString() === activeId) || null;
  });

  createEffect(() => {
    const active = activeFolder();
    if (!active) return;
    const rawName = active.display_name || active.name;
    const segments = rawName.split("/").filter(Boolean);
    setExpanded((prev) => {
      const next = new Set(prev);
      let currentPath = "";
      for (const segment of segments.slice(0, -1)) {
        currentPath = currentPath ? `${currentPath}/${segment}` : segment;
        next.add(currentPath);
      }
      return next;
    });
  });

  const toggleNode = (path: string) => {
    const next = new Set(expanded());
    if (next.has(path)) {
      next.delete(path);
    } else {
      next.add(path);
    }
    setExpanded(next);
  };

  const renderNode = (node: FolderNode) => {
    const hasChildren = node.children.size > 0;
    const isExpanded = expanded().has(node.path);
    const isActive = node.folder?.id.toString() === props.activeFolderId;

    if (hasChildren) {
      return (
        <li class="w-full">
          <details
            class="w-full"
            open={isExpanded}
            onToggle={(e) => {
              const target = e.currentTarget as HTMLDetailsElement;
              setExpanded((prev) => {
                const next = new Set(prev);
                if (target.open) {
                  next.add(node.path);
                } else {
                  next.delete(node.path);
                }
                return next;
              });
            }}
          >
            <summary class="flex items-center w-full min-w-0 px-2">
              <div class="flex items-center gap-2 flex-1 min-w-0">
                <FolderIcon
                  folderType={node.folder?.folder_type || null}
                  class="w-4 h-4"
                />

                <span
                  class="truncate text-sm"
                  classList={{ "privacy-blur": !!props.privacyEnabled }}
                  title={node.name}
                >
                  {node.name}
                </span>
              </div>

              <Show when={node.folder}>
                <span class="badge badge-sm badge-ghost ml-auto flex-shrink-0">
                  <Show
                    when={(node.folder?.unread_messages || 0) > 0}
                    fallback={node.folder?.total_messages || 0}
                  >
                    {node.folder?.unread_messages || 0}/
                    {node.folder?.total_messages || 0}
                  </Show>
                </span>
              </Show>
            </summary>

            <ul class="p-0">
              <For each={sortNodes(Array.from(node.children.values()))}>
                {(child) => renderNode(child)}
              </For>
            </ul>
          </details>
        </li>
      );
    }

    return (
      <li class="w-full">
        <Show when={node.folder && props.accountId}>
          <A
            href={`/emails/account/${props.accountId}/folder/${node.folder!.id}`}
            class="flex items-center w-full px-2"
            classList={{
              "menu-active": isActive,
              "privacy-blur": !!props.privacyEnabled,
            }}
          >
            <div class="flex items-center gap-2 flex-1 min-w-0">
              <FolderIcon
                folderType={node.folder?.folder_type || null}
                class="w-4 h-4"
              />
              <span class="truncate text-sm min-w-0" title={node.name}>
                {node.name}
              </span>
            </div>
            <span class="badge badge-sm badge-ghost ml-auto flex-shrink-0">
              <Show
                when={(node.folder?.unread_messages || 0) > 0}
                fallback={node.folder?.total_messages || 0}
              >
                {node.folder?.unread_messages || 0}/
                {node.folder?.total_messages || 0}
              </Show>
            </span>
          </A>
        </Show>
      </li>
    );
  };

  return (
    <ul class="menu w-full">
      <For each={sortNodes(Array.from(tree().children.values()))}>
        {(node) => renderNode(node)}
      </For>
    </ul>
  );
}
