import { createSignal, onMount } from "solid-js";
import { getApiUrl } from "../../config/api";
import type { AiProviderApiKeyConfig, SettingsResponse, UpdateAiProviderApiKeysRequest } from "../../api-types/types";

export default function SettingsApiKeys() {
  const [apiKeys, setApiKeys] = createSignal<AiProviderApiKeyConfig[]>([]);
  const [geminiKey, setGeminiKey] = createSignal("");
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
        setApiKeys(data.ai_provider_api_keys);
      }
    } catch (error) {
      console.error("Failed to fetch settings:", error);
    }
  };

  const saveApiKeys = async () => {
    setIsLoading(true);
    setMessage("");
    try {
      const requestBody: UpdateAiProviderApiKeysRequest = {
        gemini_api_key: geminiKey() || null,
      };

      const response = await fetch(getApiUrl("/api/settings/ai-provider-api-keys"), {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify(requestBody),
      });
      if (response.ok) {
        setMessage("API keys saved successfully!");
        setGeminiKey("");
        await fetchSettings();
      } else {
        setMessage("Failed to save API keys.");
      }
    } catch (error) {
      console.error("Failed to save API keys:", error);
      setMessage("Failed to save API keys.");
    } finally {
      setIsLoading(false);
    }
  };

  const getGeminiKey = () => apiKeys().find((k) => k.name === "gemini");

  return (
    <div class="card bg-base-100 shadow-xl">
      <div class="card-body">
        <h2 class="card-title">API Keys</h2>

        <div class="space-y-4">
          {/* Gemini API Key */}
          <div class="form-control w-full max-w-md">
            <label class="label">
              <span class="label-text">Google Gemini API Key</span>
              <span class="label-text-alt">
                {getGeminiKey()?.is_configured
                  ? "Configured"
                  : "Not configured"}
              </span>
            </label>
            {getGeminiKey()?.is_configured && getGeminiKey()?.key && (
              <div class="text-sm text-gray-500 mb-2">
                Current: {getGeminiKey()?.key}
              </div>
            )}
            <input
              type="password"
              placeholder="Enter your Gemini API key"
              class="input input-bordered w-full"
              value={geminiKey()}
              onInput={(e) => setGeminiKey(e.target.value)}
            />
          </div>
        </div>

        <div class="card-actions justify-end mt-4">
          <button
            class="btn btn-primary"
            onClick={saveApiKeys}
            disabled={isLoading()}
          >
            {isLoading() ? "Saving..." : "Save API Keys"}
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
