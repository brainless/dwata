import { getApiUrl } from "../config/api";
import type {
  CredentialMetadata,
  CredentialListResponse,
  EmailFolder,
  EmailLabel,
  Email,
  ListFoldersResponse,
  ListLabelsResponse,
  ListEmailsResponse,
  EmailsByIdsRequest,
  EmailsByIdsResponse,
  SearchResponse,
  SearchHit,
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
    // For query-based search, show account-wide results instead of forcing
    // current folder scope (which can hide valid hits from other folders).
    return searchEmails(searchQuery, limit, offset, { credentialId });
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
    // Keep search behavior consistent across views: account-wide results.
    return searchEmails(searchQuery, limit, offset, { credentialId });
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

export async function fetchEmailsByIds(emailIds: Array<number | bigint>): Promise<Email[]> {
  const payload: EmailsByIdsRequest = {
    email_ids: emailIds.map((id) => (typeof id === "bigint" ? Number(id) : id)),
  };
  const response = await fetch(getApiUrl("/api/emails/by-ids"), {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(payload),
  });
  if (!response.ok) throw new Error(`HTTP ${response.status}`);
  const data: EmailsByIdsResponse = await response.json();
  return data.emails;
}

type SearchScope = {
  credentialId?: bigint;
  folderId?: bigint;
  labelId?: bigint;
};

type IdLike = number | bigint;

function toComparableId(value: unknown): string | null {
  if (typeof value === "bigint") return value.toString();
  if (typeof value === "number") {
    return Number.isFinite(value) ? Math.trunc(value).toString() : null;
  }
  if (typeof value === "string") {
    const trimmed = value.trim();
    return trimmed.length > 0 ? trimmed : null;
  }
  return null;
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
    const typedEmail = email as unknown as {
      credential_id?: unknown;
      credentialId?: unknown;
      folder_id?: unknown;
      folderId?: unknown;
    };
    const emailCredentialId = toComparableId(
      typedEmail.credential_id ?? typedEmail.credentialId,
    );
    const emailFolderId = toComparableId(typedEmail.folder_id ?? typedEmail.folderId);

    if (
      scope.credentialId &&
      emailCredentialId !== toComparableId(scope.credentialId)
    ) {
      return false;
    }
    if (
      scope.folderId &&
      emailFolderId !== toComparableId(scope.folderId)
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
    target: "email",
    ...(scope.credentialId ? { credential_id: scope.credentialId.toString() } : {}),
    limit: "100",
    offset: "0",
  });
  const response = await fetch(getApiUrl(`/api/search?${params}`));
  if (!response.ok) throw new Error(`HTTP ${response.status}`);

  const searchData: SearchResponse = await response.json();
  const emailIds = Array.from(
    new Set(
      searchData.hits
        .map((hit: SearchHit) => {
          // HitId is serialized as { "email": <id> } or { "file": <id> }.
          const rawId = (hit.hit_id as { email?: number | bigint | string })?.email;
          if (typeof rawId === "number") return rawId;
          if (typeof rawId === "bigint") return Number(rawId);
          if (typeof rawId === "string") {
            const parsed = Number(rawId);
            return Number.isFinite(parsed) ? parsed : null;
          }
          return null;
        })
        .filter((id): id is number => id !== null),
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
            toComparableId((label as unknown as { id?: unknown }).id) ===
            toComparableId(scope.labelId),
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
