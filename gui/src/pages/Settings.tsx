import { A, useLocation } from "@solidjs/router";
import { Show } from "solid-js";
import SettingsGeneral from "./settings/General";
import SettingsApiKeys from "./settings/ApiKeys";
import SettingsAccounts from "./settings/Accounts";

export default function Settings() {
  const location = useLocation();

  // Determine active tab from URL path
  const activeTab = () => {
    const path = location.pathname;
    if (path === "/settings/api-keys") return "api-keys";
    if (path === "/settings/accounts") return "accounts";
    return "general";
  };

  return (
    <div class="p-8 min-h-screen">
      <h1 class="text-3xl font-bold mb-6">Settings</h1>

      {/* Tab Navigation */}
      <div class="tabs tabs-bordered mb-6">
        <A
          href="/settings"
          class={`tab ${activeTab() === "general" ? "tab-active" : ""}`}
        >
          General
        </A>
        <A
          href="/settings/api-keys"
          class={`tab ${activeTab() === "api-keys" ? "tab-active" : ""}`}
        >
          API Keys
        </A>
        <A
          href="/settings/accounts"
          class={`tab ${activeTab() === "accounts" ? "tab-active" : ""}`}
        >
          Accounts
        </A>
      </div>

      {/* Tab Content */}
      <div class="h-full">
        <Show when={activeTab() === "general"}>
          <SettingsGeneral />
        </Show>

        <Show when={activeTab() === "api-keys"}>
          <SettingsApiKeys />
        </Show>

        <Show when={activeTab() === "accounts"}>
          <SettingsAccounts />
        </Show>
      </div>
    </div>
  );
}
