import { createSignal, onMount, Show, For } from "solid-js";
import {
  HiOutlineFolder,
  HiOutlineTrash,
  HiOutlineEye,
  HiOutlineCheckCircle,
  HiOutlineXCircle,
} from "solid-icons/hi";
import type {
  CreateLocalFileCredentialRequest,
  LocalFileSettings,
  CredentialMetadata,
  CredentialListResponse,
} from "../../api-types/types";
import { getApiUrl } from "../../config/api";

export default function SettingsFolders() {
  const [identifier, setIdentifier] = createSignal("");
  const [filePath, setFilePath] = createSignal("");
  const [description, setDescription] = createSignal("");
  const [fileType, setFileType] = createSignal("");
  const [fileInput, setFileInput] = createSignal<HTMLInputElement | null>(null);

  const [isLoading, setIsLoading] = createSignal(false);
  const [message, setMessage] = createSignal("");
  const [messageType, setMessageType] = createSignal<"success" | "error">(
    "success",
  );

  const [credentials, setCredentials] = createSignal<CredentialMetadata[]>([]);
  const [isLoadingList, setIsLoadingList] = createSignal(true);

  onMount(async () => {
    await fetchCredentials();
  });

  const fetchCredentials = async () => {
    setIsLoadingList(true);
    try {
      const response = await fetch(getApiUrl("/api/credentials"));
      if (response.ok) {
        const data: CredentialListResponse = await response.json();
        const localFileCredentials = data.credentials.filter(
          (cred) => cred.credential_type === "localfile",
        );
        setCredentials(localFileCredentials);
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
        setMessage(`Folder "${identifier}" deleted successfully!`);
        await fetchCredentials();
      } else {
        setMessageType("error");
        setMessage(`Failed to delete folder "${identifier}".`);
      }
    } catch (error) {
      console.error("Failed to delete credential:", error);
      setMessageType("error");
      setMessage("Failed to delete folder. Please try again.");
    }
  };

  const handleSelectFolder = () => {
    fileInput()?.click();
  };

  const handleFileChange = (e: Event) => {
    const target = e.target as HTMLInputElement;
    if (target.files && target.files.length > 0) {
      const firstFile = target.files[0];
      const path = (firstFile as any).webkitRelativePath || firstFile.name;
      const folderPath = path.split("/").slice(0, -1).join("/") || path;
      setFilePath(folderPath);

      if (!identifier()) {
        const folderName = folderPath.split("/").pop() || folderPath;
        setIdentifier(folderName);
      }
    }
  };

  const handleSubmit = async (e: Event) => {
    e.preventDefault();
    setIsLoading(true);
    setMessage("");

    try {
      const settings: LocalFileSettings = {
        file_path: filePath(),
        description: description() || null,
        file_type: fileType() || null,
      };

      const request: CreateLocalFileCredentialRequest = {
        identifier: identifier(),
        settings,
        notes: null,
      };

      const response = await fetch(getApiUrl("/api/credentials"), {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({
          credential_type: "localfile",
          identifier: request.identifier,
          username: "local",
          password: null,
          service_name: null,
          port: null,
          use_tls: null,
          notes: request.notes,
          extra_metadata: JSON.stringify(settings),
        }),
      });

      if (response.ok) {
        setMessageType("success");
        setMessage("Folder added successfully!");
        setIdentifier("");
        setFilePath("");
        setDescription("");
        setFileType("");
        await fetchCredentials();
      } else {
        const error = await response.json();
        setMessageType("error");
        setMessage(error.error || "Failed to add folder.");
      }
    } catch (error) {
      console.error("Failed to add folder:", error);
      setMessageType("error");
      setMessage("Failed to add folder. Please try again.");
    } finally {
      setIsLoading(false);
    }
  };

  return (
    <div class="space-y-6">
      <div class="card bg-base-100 shadow-xl">
        <div class="card-body">
          <h2 class="card-title">Your Folders</h2>
          <p class="text-sm text-base-content/70 mb-4">
            Manage your configured local folders
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
                  <HiOutlineFolder class="w-12 h-12 mx-auto mb-2 opacity-30" />
                  <p>No folders configured yet.</p>
                  <p class="text-sm">Add your first folder below.</p>
                </div>
              }
            >
              <div class="overflow-x-auto">
                <table class="table table-zebra">
                  <thead>
                    <tr>
                      <th>Folder</th>
                      <th>Type</th>
                      <th>Description</th>
                      <th>Status</th>
                      <th></th>
                    </tr>
                  </thead>
                  <tbody>
                    <For each={credentials()}>
                      {(credential) => {
                        const settings = credential.extra_metadata
                          ? JSON.parse(credential.extra_metadata)
                          : {};

                        return (
                          <tr>
                            <td>
                              <div>
                                <div class="font-bold">
                                  {credential.identifier}
                                </div>
                                <div class="text-sm opacity-60">
                                  {settings.file_path || "N/A"}
                                </div>
                              </div>
                            </td>
                            <td>
                              <span class="badge badge-sm badge-primary gap-1">
                                <HiOutlineFolder class="w-3 h-3" />
                                Local
                              </span>
                            </td>
                            <td>
                              <div class="text-sm">
                                {settings.description ||
                                  settings.file_type ||
                                  "-"}
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
                                  title="Delete folder"
                                  onClick={() =>
                                    deleteCredential(
                                      credential.id.toString(),
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

      <div class="card bg-base-100 shadow-xl">
        <div class="card-body">
          <h2 class="card-title">Add Folder</h2>
          <p class="text-sm text-base-content/70 mb-4">
            Share a local folder path to be used by dwata. Note: Browser will show an upload confirmation but no files are actually uploaded - this is a limitation of browser APIs for folder selection.
          </p>

          <form onSubmit={handleSubmit} class="space-y-4">
            <div class="form-control w-full">
              <label class="label">
                <span class="label-text">Folder Name *</span>
                <span class="label-text-alt text-xs">Unique identifier</span>
              </label>
              <input
                type="text"
                placeholder="e.g., documents, work_files, archives"
                class="input input-bordered w-full"
                value={identifier()}
                onInput={(e) => setIdentifier(e.target.value)}
                required
              />
            </div>

            <div class="form-control w-full">
              <label class="label">
                <span class="label-text">Folder Path *</span>
                <span class="label-text-alt text-xs">
                  Use native path selection
                </span>
              </label>
              <div class="flex gap-2">
                <input
                  type="text"
                  placeholder="Click to select folder"
                  class="input input-bordered w-full"
                  value={filePath()}
                  readOnly
                  required
                />
                <button
                  type="button"
                  class="btn btn-primary"
                  onClick={handleSelectFolder}
                >
                  <HiOutlineFolder class="w-5 h-5" />
                  Browse
                </button>
                <input
                  type="file"
                  webkitdirectory
                  directory
                  class="hidden"
                  ref={setFileInput}
                  onChange={handleFileChange}
                />
              </div>
            </div>

            <div class="form-control w-full">
              <label class="label">
                <span class="label-text">Description (Optional)</span>
              </label>
              <input
                type="text"
                placeholder="What does this folder contain?"
                class="input input-bordered w-full"
                value={description()}
                onInput={(e) => setDescription(e.target.value)}
              />
            </div>

            <div class="form-control w-full">
              <label class="label">
                <span class="label-text">File Type Hint (Optional)</span>
                <span class="label-text-alt text-xs">
                  e.g., linkedin-archive, email-export
                </span>
              </label>
              <input
                type="text"
                placeholder="Type of files in this folder"
                class="input input-bordered w-full"
                value={fileType()}
                onInput={(e) => setFileType(e.target.value)}
              />
            </div>

            {message() && (
              <div
                class={`alert ${
                  messageType() === "success" ? "alert-success" : "alert-error"
                }`}
              >
                <span>{message()}</span>
              </div>
            )}

            <div class="card-actions justify-end">
              <button
                type="submit"
                class="btn btn-primary btn-wide"
                disabled={isLoading()}
              >
                {isLoading() ? (
                  <>
                    <span class="loading loading-spinner loading-sm"></span>
                    Adding Folder...
                  </>
                ) : (
                  "Add Folder"
                )}
              </button>
            </div>
          </form>
        </div>
      </div>
    </div>
  );
}
