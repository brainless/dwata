import { Component, createContext, useContext } from "solid-js";
import { createStore } from "solid-js/store";
import { IProviderPropTypes, IWorkspace } from "../utils/types";
import { invoke } from "@tauri-apps/api/core";
import { Module } from "../api_types/Module";
import { ModuleDataReadList } from "../api_types/ModuleDataReadList";

const makeStore = () => {
  const [store, setStore] = createStore<IWorkspace>({
    DirectorySource: [],
    DatabaseSource: [],
    AIIntegration: [],
    OAuth2App: [],
    EmailAccount: [],

    isReady: {},
    isFetching: {},
  });

  return [
    store,
    {
      readModuleList: async (module: Module) => {
        if (module in store.isFetching && !!store.isFetching[module]) {
          return;
        }
        setStore((state) => ({
          ...state,
          isFetching: {
            ...state.isFetching,
            [module]: true,
          },
        }));

        // We invoke the Tauri API to load data
        const response: ModuleDataReadList = await invoke<ModuleDataReadList>(
          "read_row_list_for_module",
          {
            module,
          },
        );
        if (response["type"] === module) {
          setStore((state) => ({
            ...state,
            [module]: response["data"],
            isReady: {
              ...state.isReady,
              [module]: true,
            },
            isFetching: {
              ...state.isFetching,
              [module]: false,
            },
          }));
        }
      },
    },
  ] as const; // `as const` forces tuple type inference
};

type TStoreAndFunctions = ReturnType<typeof makeStore>;
export const workspaceStore = makeStore();

const WorkspaceContext = createContext<TStoreAndFunctions>(workspaceStore);

export const WorkspaceProvider: Component<IProviderPropTypes> = (props) => {
  return (
    <WorkspaceContext.Provider value={workspaceStore}>
      {props.children}
    </WorkspaceContext.Provider>
  );
};

export const useWorkspace = () => useContext(WorkspaceContext);
