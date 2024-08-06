import { Component, createContext, createResource, useContext } from "solid-js";
import { invoke } from "@tauri-apps/api/core";
import { IProviderPropTypes } from "../utils/types";
import { ModuleDataReadList } from "../api_types/ModuleDataReadList";
import { Module } from "../api_types/Module";
import { Email } from "../api_types/Email";
import { ModuleFilters } from "../api_types/ModuleFilters";
import { EmailAccount } from "../api_types/EmailAccount";
import { Mailbox } from "../api_types/Mailbox";

const makeStore = () => {
  const [emails, { mutate: _mutateEmails, refetch: refetchEmails }] =
    createResource<{
      data: Array<Email>;
    }>(async (_, { value, refetching }) => {
      if (
        !!refetching &&
        typeof refetching === "object" &&
        "mailboxId" in refetching
      ) {
        // We are fetching emails from a specific mailbox
        console.info("invoke search_emails {}", refetching);
        const result = await invoke<ModuleDataReadList>("search_emails", {
          module: "Email" as Module,
          filters: {
            Email: {
              mailboxId: refetching.mailboxId,
              searchQuery:
                "searchQuery" in refetching ? refetching.searchQuery : null,
            },
          } as ModuleFilters,
        });
        if (!!result && "type" in result && result["type"] === "Email") {
          console.log(result["data"]);

          return { ...value, data: result["data"] as Array<Email> };
        }
      } else {
        // We are fetching emails from all mailboxes
        console.log("Fetching emails from all mailboxes");
      }
      return { data: [] };
    });

  const [
    emailAccounts,
    { mutate: _mutateEmailAccounts, refetch: refetchEmailAccounts },
  ] = createResource<{ data: Array<EmailAccount> }>(
    async (_, { value: _value, refetching: _refetching }) => {
      const result = await invoke<ModuleDataReadList>(
        "read_row_list_for_module",
        {
          module: "EmailAccount" as Module,
        },
      );

      if (!!result && "type" in result && result["type"] === "EmailAccount") {
        return {
          data: result["data"] as Array<EmailAccount>,
        };
      }
      return { data: [] };
    },
  );

  const [mailboxes, { mutate: _mutateMailboxes, refetch: refetchMailboxes }] =
    createResource<{ data: Array<Mailbox> }>(
      async (_, { value: _value, refetching: _refetching }) => {
        const result = await invoke<ModuleDataReadList>(
          "read_row_list_for_module",
          {
            module: "Mailbox" as Module,
          },
        );
        if (!!result && "type" in result && result["type"] === "Mailbox") {
          return {
            data: result["data"] as Array<Mailbox>,
          };
        }
        return { data: [] };
      },
    );

  return [
    emailAccounts,
    mailboxes,
    emails,
    {
      fetchAllEmailAccounts: () => {
        refetchEmailAccounts();
      },

      fetchAllMailboxes: () => {
        refetchMailboxes();
      },

      fetchEmailsForMailbox: (
        mailboxId: number,
        searchQuery: string | undefined,
      ) => {
        if (!mailboxId) return;
        console.log("Fetching search results for: ", searchQuery);

        refetchEmails({ mailboxId, searchQuery });
      },
    },
  ] as const; // `as const` forces tuple type inference
};

type TStoreAndFunctions = ReturnType<typeof makeStore>;
const emailStore = makeStore();

const EmailContext = createContext<TStoreAndFunctions>(emailStore);

export const EmailProvider: Component<IProviderPropTypes> = (props) => {
  return (
    <EmailContext.Provider value={emailStore}>
      {props.children}
    </EmailContext.Provider>
  );
};

export const useEmail = () => useContext(EmailContext);
