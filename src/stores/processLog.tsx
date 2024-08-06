import { Component, createContext, onMount, useContext } from "solid-js";
import { createStore } from "solid-js/store";
import { IProviderPropTypes } from "../utils/types";
import { listen } from "@tauri-apps/api/event";
import { invoke } from "@tauri-apps/api/core";

interface IStore {
  isRunning: boolean;
}

const makeStore = () => {
  const [store, setStore] = createStore<IStore>({
    isRunning: false,
  });

  return [
    store,
    {
      start: async () => {
        if (store.isRunning) return;
        setStore({ ...store, isRunning: true });
        invoke("get_process_log");
        await listen("process_log", async (event) => {
          console.log(event.event, JSON.parse(event.payload as string));
        });
      },
    },
  ] as const; // `as const` forces tuple type inference
};

type TStoreAndFunctions = ReturnType<typeof makeStore>;
export const processLogStore = makeStore();

const AppStatusContext = createContext<TStoreAndFunctions>(processLogStore);

export const ProcessLogProvider: Component<IProviderPropTypes> = (props) => {
  return (
    <AppStatusContext.Provider value={processLogStore}>
      {props.children}
    </AppStatusContext.Provider>
  );
};

export const useProcessLog = () => useContext(AppStatusContext);

export const ProcessLogLoader: Component = () => {
  const [_, { start }] = useProcessLog();
  // const [_p, { mutate: _m, refetch }] = createResource(async () => {
  //   await listenToProcessLog();
  // });

  onMount(() => {
    start();
    // refetch();
  });

  return null;
};
