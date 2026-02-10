import { A, useLocation } from "@solidjs/router";
import { Show } from "solid-js";
import SettingsGeneral from "./settings/General";
import SettingsApiKeys from "./settings/ApiKeys";
import SettingsOAuthClientApps from "./settings/OAuthClientApps";
import SettingsAccounts from "./settings/Accounts";
import SettingsFolders from "./settings/Folders";
import SettingsPrivacy from "./settings/Privacy";

export default function Settings() {
  const location = useLocation();

  // Determine active tab from URL path
  const activeTab = () => {
    const path = location.pathname;
    if (path === "/settings/ai-providers") return "ai-providers";
    if (path === "/settings/oauth-apps") return "oauth-apps";
    if (path === "/settings/accounts") return "accounts";
    if (path === "/settings/folders") return "folders";
    if (path === "/settings/privacy") return "privacy";
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
          href="/settings/ai-providers"
          class={`tab ${activeTab() === "ai-providers" ? "tab-active" : ""}`}
        >
          AI Providers
        </A>
        <A
          href="/settings/oauth-apps"
          class={`tab ${activeTab() === "oauth-apps" ? "tab-active" : ""}`}
        >
          OAuth Client Apps
        </A>
        <A
          href="/settings/accounts"
          class={`tab ${activeTab() === "accounts" ? "tab-active" : ""}`}
        >
          Email Accounts
        </A>
        <A
          href="/settings/privacy"
          class={`tab ${activeTab() === "privacy" ? "tab-active" : ""}`}
        >
          Privacy
        </A>
      </div>

      {/* Tab Content */}
      <div class="h-full">
        <Show when={activeTab() === "general"}>
          <SettingsGeneral />
        </Show>

        <Show when={activeTab() === "ai-providers"}>
          <SettingsApiKeys />
        </Show>

        <Show when={activeTab() === "oauth-apps"}>
          <SettingsOAuthClientApps />
        </Show>

        <Show when={activeTab() === "accounts"}>
          <SettingsAccounts />
        </Show>

        <Show when={activeTab() === "folders"}>
          <SettingsFolders />
        </Show>

        <Show when={activeTab() === "privacy"}>
          <SettingsPrivacy />
        </Show>
      </div>
    </div>
  );
}
