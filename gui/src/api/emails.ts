import { getApiUrl } from "../config/api";
import type {
  CredentialMetadata,
  CredentialListResponse,
  Document,
  EmailFolder,
  EmailLabel,
  Email,
  ListFoldersResponse,
  ListLabelsResponse,
  ListEmailsResponse,
  SearchDocumentsResponse,
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
  credentialId: bigint,
  folderId: bigint,
  limit: number = 50,
  offset: number = 0,
  searchQuery?: string
): Promise<ListEmailsResponse> {
  if (searchQuery && searchQuery.trim()) {
    return searchEmails(searchQuery, limit, offset, { credentialId, folderId });
  }
  const params = new URLSearchParams({
    folder_id: folderId.toString(),
    limit: limit.toString(),
    offset: offset.toString(),
  });
  const response = await fetch(getApiUrl(`/api/emails?${params}`));
  if (!response.ok) throw new Error(`HTTP ${response.status}`);
  return await response.json();
}

export async function fetchEmailsByLabel(
  credentialId: bigint,
  labelId: bigint,
  limit: number = 50,
  offset: number = 0,
  searchQuery?: string
): Promise<ListEmailsResponse> {
  if (searchQuery && searchQuery.trim()) {
    return searchEmails(searchQuery, limit, offset, { credentialId, labelId });
  }
  const params = new URLSearchParams({
    label_id: labelId.toString(),
    limit: limit.toString(),
    offset: offset.toString(),
  });
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
  if (searchQuery && searchQuery.trim()) {
    return searchEmails(searchQuery, limit, offset, { credentialId });
  }
  const params = new URLSearchParams({
    credential_id: credentialId.toString(),
    limit: limit.toString(),
    offset: offset.toString(),
  });
  const response = await fetch(getApiUrl(`/api/emails?${params}`));
  if (!response.ok) throw new Error(`HTTP ${response.status}`);
  return await response.json();
}

type SearchScope = {
  credentialId?: bigint;
  folderId?: bigint;
  labelId?: bigint;
};

type IdLike = number | bigint;

function toNumericId(value: IdLike): number {
  return typeof value === "bigint" ? Number(value) : value;
}

async function fetchEmail(emailId: IdLike): Promise<Email> {
  const response = await fetch(getApiUrl(`/api/emails/${emailId.toString()}`));
  if (!response.ok) throw new Error(`HTTP ${response.status}`);
  return await response.json();
}

async function fetchEmailLabels(emailId: IdLike): Promise<Array<{ id: IdLike }>> {
  const response = await fetch(getApiUrl(`/api/emails/${emailId.toString()}/labels`));
  if (!response.ok) throw new Error(`HTTP ${response.status}`);
  return await response.json();
}

function applyScopeFilter(emails: Email[], scope: SearchScope): Email[] {
  return emails.filter((email) => {
    if (
      scope.credentialId &&
      toNumericId(email.credential_id as IdLike) !== toNumericId(scope.credentialId)
    ) {
      return false;
    }
    if (
      scope.folderId &&
      toNumericId(email.folder_id as IdLike) !== toNumericId(scope.folderId)
    ) {
      return false;
    }
    return true;
  });
}

async function searchEmails(
  searchQuery: string,
  limit: number,
  offset: number,
  scope: SearchScope,
): Promise<ListEmailsResponse> {
  const params = new URLSearchParams({
    q: searchQuery,
    kind: "email",
    ...(scope.credentialId ? { credential_id: scope.credentialId.toString() } : {}),
    limit: "100",
    offset: "0",
  });
  const response = await fetch(getApiUrl(`/api/documents/search?${params}`));
  if (!response.ok) throw new Error(`HTTP ${response.status}`);

  const searchData: SearchDocumentsResponse = await response.json();
  const emailIds = Array.from(
    new Set(
      searchData.documents
        .map((doc: Document) => doc.email_id)
        .filter((emailId): emailId is IdLike => emailId !== null),
    ),
  );

  const hydratedEmails = await Promise.all(emailIds.map((emailId) => fetchEmail(emailId)));
  let scoped = applyScopeFilter(hydratedEmails, scope);

  if (scope.labelId) {
    const byEmailLabels = await Promise.all(
      scoped.map(async (email) => ({
        email,
        labels: await fetchEmailLabels(email.id),
      })),
    );
    scoped = byEmailLabels
      .filter((row) =>
        row.labels.some(
          (label) =>
            toNumericId(label.id as IdLike) === toNumericId(scope.labelId as IdLike),
        ),
      )
      .map((row) => row.email);
  }

  const page = scoped.slice(offset, offset + limit);
  return {
    emails: page,
    total_count: BigInt(scoped.length),
    has_more: offset + limit < scoped.length,
  };
}
