import { A } from "@solidjs/router";
import { For, Show, createResource } from "solid-js";
import type { ListFinancialTemplatesResponse } from "../api-types/types";
import { getApiUrl } from "../config/api";
import FinancialPageLayout from "../components/FinancialPageLayout";
import { createSignal } from "solid-js";

async function fetchTemplates(): Promise<ListFinancialTemplatesResponse> {
  const response = await fetch(getApiUrl("/api/financial/templates?limit=200"));
  if (!response.ok) throw new Error(`HTTP ${response.status}`);
  return await response.json();
}

function snippet(text: string, max: number): string {
  const normalized = text.replace(/\s+/g, " ").trim();
  if (normalized.length <= max) return normalized;
  return `${normalized.slice(0, max - 1)}...`;
}

function formatTemplateBody(text: string): string {
  return text
    .replace(/\r\n/g, "\n")
    .replace(/\n\s*-{3,}\s*\n/g, "\n\n")
    .replace(/(Subject:[^\n]*)\n+(Body:)/i, "$1\n\n$2")
    .trim();
}

function templateSnippet(text: string, max: number): string {
  const formatted = formatTemplateBody(text);
  if (formatted.length <= max) return formatted;
  return `${formatted.slice(0, max - 1)}...`;
}

export default function FinancialTemplates() {
  const [templates, { refetch }] = createResource(fetchTemplates);
  const [selected, setSelected] = createSignal<Set<string>>(new Set());
  const [deleting, setDeleting] = createSignal(false);
  const [deleteError, setDeleteError] = createSignal<string | null>(null);

  const rows = () => templates()?.templates ?? [];
  const selectedCount = () => selected().size;
  const allVisibleSelected = () =>
    rows().length > 0 && rows().every((r) => selected().has(r.template.id.toString()));

  const toggleRow = (templateId: bigint, checked: boolean) => {
    const key = templateId.toString();
    setSelected((prev) => {
      const next = new Set(prev);
      if (checked) next.add(key);
      else next.delete(key);
      return next;
    });
  };

  const toggleAll = (checked: boolean) => {
    setSelected(() => {
      if (!checked) return new Set<string>();
      return new Set(rows().map((r) => r.template.id.toString()));
    });
  };

  const deleteSelected = async () => {
    if (selectedCount() === 0) return;
    setDeleting(true);
    setDeleteError(null);
    try {
      const templateIds = Array.from(selected()).map((id) => Number(id));
      const response = await fetch(getApiUrl("/api/financial/templates"), {
        method: "DELETE",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ template_ids: templateIds }),
      });
      if (!response.ok) throw new Error(`HTTP ${response.status}`);
      setSelected(new Set());
      await refetch();
    } catch (err) {
      setDeleteError(err instanceof Error ? err.message : "Failed to delete templates");
    } finally {
      setDeleting(false);
    }
  };

  const footerActions = (
    <>
      <A href="/financial" class="btn btn-ghost btn-sm">
        Back to Financial
      </A>
      <A href="/financial/templates/detect" class="btn btn-secondary btn-sm">
        Detect Templates
      </A>
    </>
  );

  return (
    <FinancialPageLayout
      title="Financial Overview: Templates"
      subtitle="Detected templates and variable mappings."
      footer={footerActions}
    >
      <div class="card bg-base-100 shadow overflow-hidden">
        <div class="card-body">
          <h2 class="card-title">Saved Templates</h2>
          <Show when={!templates.loading} fallback={<div class="text-sm text-base-content/70">Loading templates...</div>}>
            <Show when={!templates.error} fallback={<div class="alert alert-error"><span>Failed to load templates.</span></div>}>
              <Show when={deleteError()}>
                <div class="alert alert-error mb-3">
                  <span>{deleteError()}</span>
                </div>
              </Show>
              <Show
                when={(templates()?.templates.length ?? 0) > 0}
                fallback={<div class="text-sm text-base-content/70">No templates yet. Run detection to generate templates.</div>}
              >
                <div class="overflow-x-auto">
                  <table class="table table-zebra table-sm">
                    <thead>
                      <tr>
                        <th>
                          <input
                            type="checkbox"
                            class="checkbox checkbox-sm"
                            checked={allVisibleSelected()}
                            onChange={(e) => toggleAll(e.currentTarget.checked)}
                          />
                        </th>
                        <th>Sender</th>
                        <th>Type</th>
                        <th>Template Snippet</th>
                        <th>Variables</th>
                      </tr>
                    </thead>
                    <tbody>
                      <For each={templates()?.templates ?? []}>
                        {(row) => (
                          <tr>
                            <td>
                              <input
                                type="checkbox"
                                class="checkbox checkbox-sm"
                                checked={selected().has(row.template.id.toString())}
                                onChange={(e) => toggleRow(row.template.id, e.currentTarget.checked)}
                              />
                            </td>
                            <td class="max-w-48 truncate" title={row.template.data_source_id}>
                              {row.template.data_source_id}
                            </td>
                            <td class="capitalize">{row.template.template_type}</td>
                            <td class="max-w-xl whitespace-pre-wrap" title={formatTemplateBody(row.template.template_body)}>
                              {templateSnippet(row.template.template_body, 1000)}
                            </td>
                            <td>
                              <Show
                                when={row.variables.length > 0}
                                fallback={<span class="text-xs text-base-content/60">No variables</span>}
                              >
                                <div class="space-y-1">
                                  <For each={row.variables}>
                                    {(v) => (
                                      <div class="text-xs leading-tight" title={`${v.placeholder_name} -> ${v.target_field}`}>
                                        {snippet(`${v.placeholder_name} -> ${v.target_field}`, 42)}
                                      </div>
                                    )}
                                  </For>
                                </div>
                              </Show>
                            </td>
                          </tr>
                        )}
                      </For>
                    </tbody>
                  </table>
                </div>
                <div class="mt-3 flex justify-end">
                  <button
                    class="btn btn-error btn-sm"
                    disabled={selectedCount() === 0 || deleting()}
                    onClick={deleteSelected}
                  >
                    {deleting() ? "Deleting..." : "Delete Selected"}
                  </button>
                </div>
              </Show>
            </Show>
          </Show>
        </div>
      </div>
    </FinancialPageLayout>
  );
}
