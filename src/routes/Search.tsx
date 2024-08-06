import {
  Component,
  For,
  createComputed,
  createMemo,
  createResource,
  createSignal,
  onMount,
} from "solid-js";
import Heading from "../widgets/typography/Heading";
import { useUserInterface } from "../stores/userInterface";
import TextInput from "../widgets/interactable/TextInput";
import { IFormFieldValue } from "../utils/types";
import { useSearchParams } from "@solidjs/router";
import { EmailAccount } from "../api_types/EmailAccount";
import { EmailProvider, useEmail } from "../stores/email";
import { Mailbox } from "../api_types/Mailbox";
import { Email } from "../api_types/Email";
import { invoke } from "@tauri-apps/api/core";

const SearchResultItem: Component<Email> = (props) => {
  const [_, { getColors }] = useUserInterface();
  return (
    <div
      class="text-lg font-normal border border-b-0 p-1"
      style={{
        color: getColors().colors["editor.foreground"],
        "border-color": getColors().colors["editorWidget.border"],
      }}
    >
      {props.uid}: {props.subject}
    </div>
  );
};

const MailboxLabel: Component<Mailbox> = (props) => {
  return <div>{props.name}</div>;
};

const EmailAccountBox: Component = () => {
  const [emailAccounts, mailboxes] = useEmail();
  const [searchParams] = useSearchParams();

  const getEmailAccount = createMemo<EmailAccount | undefined>(() => {
    if (!!searchParams.emailAccountId && emailAccounts.state === "ready") {
      return emailAccounts().data.find(
        (x) => x.id === parseInt(searchParams.emailAccountId!),
      );
    }
    return undefined;
  });

  const getMailboxes = createMemo<Array<Mailbox>>(() => {
    if (!!searchParams.emailAccountId && mailboxes.state === "ready") {
      return mailboxes().data.filter(
        (x) => x.emailAccountId === parseInt(searchParams.emailAccountId!),
      );
    }
    return [];
  });

  if (
    !searchParams.emailAccountId ||
    emailAccounts.state !== "ready" ||
    mailboxes.state !== "ready"
  ) {
    return null;
  }

  return (
    <>
      <Heading size="xl">{getEmailAccount()?.emailAddress}</Heading>
      <For each={getMailboxes()}>{(item) => <MailboxLabel {...item} />}</For>
    </>
  );
};

const Search: Component = () => {
  const [_, { getColors }] = useUserInterface();
  const [searchParams] = useSearchParams();
  const [
    _emailAccounts,
    mailboxes,
    emails,
    { fetchAllEmailAccounts, fetchAllMailboxes, fetchEmailsForMailbox },
  ] = useEmail();
  const [formData, setFormData] = createSignal<{
    [key: string]: IFormFieldValue;
  }>({});
  const [_r, { mutate: _m, refetch: _refetchEmails }] = createResource(
    async () => {
      if (!!searchParams.emailAccountId) {
        await invoke("fetch_emails", {
          pk: parseInt(searchParams.emailAccountId),
        });
      }
    },
  );

  // const [results, { mutate: _m, refetch: refetchSearch }] =
  //   createResource<SearchResult>(async () => {
  //     let results: SearchResult = {
  //       found: 0,
  //       hits: [],
  //     };
  //     if (!!formData().query && (formData()!.query! as string).length >= 1) {
  //       const pkList = !!searchParams.emailAccountId
  //         ? [parseInt(searchParams.emailAccountId)]
  //         : workspace.EmailAccount.map((x) => x.id);
  //       results = await invoke<SearchResult>("search_emails", {
  //         pkList,
  //         query: formData().query,
  //       });
  //     }
  //     return results;
  //   });

  onMount(() => {
    // refetchEmails();
  });

  createComputed(() => {
    if (!!searchParams.emailAccountId) {
      // refetchEmails();
    }
  });

  createComputed(() => {
    if (!!searchParams.emailAccountId) {
      // Load recent emails
      if (mailboxes.state === "ready" && mailboxes().data.length > 0) {
        const mailbox = mailboxes().data.find(
          (x) => x.name.toLowerCase() === "sent",
        );
        const searchQuery =
          !!formData().query && (formData()!.query! as string).length >= 1
            ? `${formData().query}`.trim()
            : undefined;
        if (mailbox) {
          fetchEmailsForMailbox(mailbox.id, searchQuery);
        }
      }
    }
  });

  const handleChange = (name: string, value: IFormFieldValue) => {
    setFormData((state) => ({
      ...state,
      [name]: value,
    }));

    // Load search results
    if (mailboxes.state === "ready" && mailboxes().data.length > 0) {
      const mailbox = mailboxes().data.find(
        (x) => x.name.toLowerCase() === "sent",
      );
      const searchQuery =
        !!formData().query && (formData()!.query! as string).length >= 1
          ? `${formData().query}`.trim()
          : undefined;
      if (mailbox) {
        fetchEmailsForMailbox(mailbox.id, searchQuery);
      }
    }
  };

  return (
    <div class="flex flex-row gap-4">
      <div
        class="max-w-96"
        style={{ color: getColors().colors["editor.foreground"] }}
      >
        <EmailAccountBox />
      </div>

      <div class="grow max-w-screen-md">
        <Heading size="2xl">Search</Heading>
        <div class="mb-4" />
        <TextInput
          name="query"
          contentType="Text"
          contentSpec={{}}
          isEditable
          onChange={handleChange}
          value={"query" in formData() ? formData().query : undefined}
        />

        <div class="mt-8">
          <For each={emails()?.data}>
            {(result) => <SearchResultItem {...result} />}
          </For>
        </div>
      </div>
    </div>
  );
};

const SearchWrapper: Component = () => {
  return (
    <EmailProvider>
      <Search />
    </EmailProvider>
  );
};

export default SearchWrapper;
