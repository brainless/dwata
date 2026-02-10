import { createSignal, onMount } from "solid-js";
import { getApiUrl } from "../../config/api";
import type {
  OAuthClientAppConfig,
  SettingsResponse,
  UpdateOAuthClientAppsRequest,
} from "../../api-types/types";

export default function SettingsOAuthClientApps() {
  const [oauthApps, setOAuthApps] = createSignal<OAuthClientAppConfig[]>([]);
  const [googleClientId, setGoogleClientId] = createSignal("");
  const [googleClientSecret, setGoogleClientSecret] = createSignal("");
  const [isLoading, setIsLoading] = createSignal(false);
  const [message, setMessage] = createSignal("");

  onMount(async () => {
    await fetchSettings();
  });

  const fetchSettings = async () => {
    try {
      const response = await fetch(getApiUrl("/api/settings"));
      if (response.ok) {
        const data: SettingsResponse = await response.json();
        setOAuthApps(data.oauth_client_apps);
      }
    } catch (error) {
      console.error("Failed to fetch settings:", error);
    }
  };

  const saveOAuthApps = async () => {
    setIsLoading(true);
    setMessage("");
    try {
      const requestBody: UpdateOAuthClientAppsRequest = {
        google_client_id: googleClientId() || null,
        google_client_secret: googleClientSecret() || null,
      };

      const response = await fetch(
        getApiUrl("/api/settings/oauth-client-apps"),
        {
          method: "POST",
          headers: {
            "Content-Type": "application/json",
          },
          body: JSON.stringify(requestBody),
        }
      );
      if (response.ok) {
        setMessage("OAuth client apps saved successfully!");
        setGoogleClientId("");
        setGoogleClientSecret("");
        await fetchSettings();
      } else {
        setMessage("Failed to save OAuth client apps.");
      }
    } catch (error) {
      console.error("Failed to save OAuth client apps:", error);
      setMessage("Failed to save OAuth client apps.");
    } finally {
      setIsLoading(false);
    }
  };

  const getGoogleApp = () =>
    oauthApps().find((app) => app.provider === "google");

  return (
    <div class="card bg-base-100 shadow-xl">
      <div class="card-body">
        <h2 class="card-title">OAuth Client Apps</h2>
        <p class="text-sm text-gray-600 mb-4">
          Configure custom OAuth applications for connecting to third-party
          services. By default, Dwata uses its own OAuth app. You can override
          this with your own OAuth client credentials.
        </p>

        <div class="space-y-4">
          {/* Google OAuth */}
          <div class="form-control w-full max-w-md">
            <label class="label">
              <span class="label-text font-semibold">Google OAuth 2.0</span>
              <span class="label-text-alt">
                {getGoogleApp()?.is_configured
                  ? "Custom app configured"
                  : "Using default Dwata app"}
              </span>
            </label>

            {/* Client ID */}
            <label class="label">
              <span class="label-text">Client ID</span>
            </label>
            {getGoogleApp()?.is_configured && getGoogleApp()?.client_id && (
              <div class="text-sm text-gray-500 mb-2">
                Current: {getGoogleApp()?.client_id}
              </div>
            )}
            <input
              type="text"
              placeholder="Enter your Google OAuth Client ID"
              class="input input-bordered w-full"
              value={googleClientId()}
              onInput={(e) => setGoogleClientId(e.target.value)}
            />

            {/* Client Secret */}
            <label class="label mt-2">
              <span class="label-text">Client Secret</span>
            </label>
            {getGoogleApp()?.is_configured && getGoogleApp()?.client_secret && (
              <div class="text-sm text-gray-500 mb-2">
                Current: {getGoogleApp()?.client_secret}
              </div>
            )}
            <input
              type="password"
              placeholder="Enter your Google OAuth Client Secret"
              class="input input-bordered w-full"
              value={googleClientSecret()}
              onInput={(e) => setGoogleClientSecret(e.target.value)}
            />

            <div class="alert alert-info mt-3">
              <svg
                xmlns="http://www.w3.org/2000/svg"
                fill="none"
                viewBox="0 0 24 24"
                class="stroke-current shrink-0 w-6 h-6"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  stroke-width="2"
                  d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z"
                />
              </svg>
              <span class="text-sm">
                Get OAuth credentials from{" "}
                <a
                  href="https://console.cloud.google.com/apis/credentials"
                  target="_blank"
                  rel="noopener noreferrer"
                  class="link link-primary"
                >
                  Google Cloud Console
                </a>
                . Configure authorized redirect URI to match your server
                address.
              </span>
            </div>
          </div>
        </div>

        <div class="card-actions justify-end mt-4">
          <button
            class="btn btn-primary"
            onClick={saveOAuthApps}
            disabled={isLoading()}
          >
            {isLoading() ? "Saving..." : "Save OAuth Apps"}
          </button>
        </div>
        {message() && (
          <div
            class={`alert mt-4 ${message().includes("success") ? "alert-success" : "alert-error"}`}
          >
            <span>{message()}</span>
          </div>
        )}
      </div>
    </div>
  );
}
