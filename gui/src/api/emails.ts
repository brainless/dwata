import { getApiUrl } from "../config/api";
import type {
  CredentialMetadata,
  CredentialListResponse,
  EmailFolder,
  EmailLabel,
  ListFoldersResponse,
  ListLabelsResponse,
  ListEmailsResponse,
} from "../api-types/types";

export async function fetchCredentials(): Promise<CredentialMetadata[]> {
  const response = await fetch(getApiUrl("/api/credentials"));
  if (!response.ok) throw new Error(`HTTP ${response.status}`);
  const data: CredentialListResponse = await response.json();
  // Filter to only IMAP/OAuth accounts
  return data.credentials.filter(
    c => c.credential_type === 'imap' || c.credential_type === 'oauth'
  );
}

export async function fetchFolders(credentialId: bigint): Promise<EmailFolder[]> {
  const response = await fetch(
    getApiUrl(`/api/credentials/${credentialId}/folders`)
  );
  if (!response.ok) throw new Error(`HTTP ${response.status}`);
  const data: ListFoldersResponse = await response.json();
  return data.folders;
}

export async function fetchLabels(credentialId: bigint): Promise<EmailLabel[]> {
  const response = await fetch(
    getApiUrl(`/api/credentials/${credentialId}/labels`)
  );
  if (!response.ok) throw new Error(`HTTP ${response.status}`);
  const data: ListLabelsResponse = await response.json();
  return data.labels;
}

export async function fetchEmailsByFolder(
  folderId: bigint,
  limit: number = 50,
  offset: number = 0,
  searchQuery?: string
): Promise<ListEmailsResponse> {
  const params = new URLSearchParams({
    folder_id: folderId.toString(),
    limit: limit.toString(),
    offset: offset.toString(),
  });
  if (searchQuery && searchQuery.trim()) {
    params.append('search_query', searchQuery);
  }
  const response = await fetch(getApiUrl(`/api/emails?${params}`));
  if (!response.ok) throw new Error(`HTTP ${response.status}`);
  return await response.json();
}

export async function fetchEmailsByLabel(
  labelId: bigint,
  limit: number = 50,
  offset: number = 0,
  searchQuery?: string
): Promise<ListEmailsResponse> {
  const params = new URLSearchParams({
    label_id: labelId.toString(),
    limit: limit.toString(),
    offset: offset.toString(),
  });
  if (searchQuery && searchQuery.trim()) {
    params.append('search_query', searchQuery);
  }
  const response = await fetch(getApiUrl(`/api/emails?${params}`));
  if (!response.ok) throw new Error(`HTTP ${response.status}`);
  return await response.json();
}

export async function fetchEmailsByAccount(
  credentialId: bigint,
  limit: number = 50,
  offset: number = 0,
  searchQuery?: string
): Promise<ListEmailsResponse> {
  const params = new URLSearchParams({
    credential_id: credentialId.toString(),
    limit: limit.toString(),
    offset: offset.toString(),
  });
  if (searchQuery && searchQuery.trim()) {
    params.append('search_query', searchQuery);
  }
  const response = await fetch(getApiUrl(`/api/emails?${params}`));
  if (!response.ok) throw new Error(`HTTP ${response.status}`);
  return await response.json();
}
