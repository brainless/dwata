import type { JSX } from "solid-js";
import { createContext, createSignal, useContext } from "solid-js";

type PrivacyModeContextValue = {
  isEnabled: () => boolean;
  setEnabled: (enabled: boolean) => void;
};

const PrivacyModeContext = createContext<PrivacyModeContextValue>();

export function PrivacyModeProvider(props: { children: JSX.Element }) {
  const [isEnabled, setEnabled] = createSignal(true);

  return (
    <PrivacyModeContext.Provider value={{ isEnabled, setEnabled }}>
      {props.children}
    </PrivacyModeContext.Provider>
  );
}

export function usePrivacyMode(): PrivacyModeContextValue {
  const ctx = useContext(PrivacyModeContext);
  if (!ctx) {
    throw new Error("usePrivacyMode must be used within PrivacyModeProvider");
  }
  return ctx;
}
