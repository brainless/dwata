import { createSignal, onMount } from "solid-js";
import { getApiUrl } from "../../config/api";
import type {
  AiProviderApiKeyConfig,
  OllamaModelsResponse,
  OllamaStatusResponse,
  SettingsResponse,
  UpdateAiProviderApiKeysRequest,
} from "../../api-types/types";

const OLLAMA_MODEL_ID = "ministral-3:3b";
const OLLAMA_MODEL_LABEL = "Ministral 3:3b";

export default function SettingsApiKeys() {
  const [apiKeys, setApiKeys] = createSignal<AiProviderApiKeyConfig[]>([]);
  const [openaiKey, setOpenaiKey] = createSignal("");
  const [geminiKey, setGeminiKey] = createSignal("");
  const [isLoading, setIsLoading] = createSignal(false);
  const [message, setMessage] = createSignal("");
  const [ollamaRunning, setOllamaRunning] = createSignal<boolean | null>(null);
  const [ollamaModels, setOllamaModels] = createSignal<string[]>([]);
  const [ollamaLoading, setOllamaLoading] = createSignal(false);
  const [ollamaMessage, setOllamaMessage] = createSignal("");
  const [ollamaError, setOllamaError] = createSignal("");

  onMount(async () => {
    await fetchSettings();
    await fetchOllamaStatus();
    await fetchOllamaModels();
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
        openai_api_key: openaiKey() || null,
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
        setOpenaiKey("");
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
  const getOpenaiKey = () => apiKeys().find((k) => k.name === "openai");
  const hasOllamaModel = () =>
    ollamaModels().some((name) => name === OLLAMA_MODEL_ID);

  const fetchOllamaStatus = async () => {
    try {
      const response = await fetch(getApiUrl("/api/ollama/status"));
      if (response.ok) {
        const data: OllamaStatusResponse = await response.json();
        setOllamaRunning(data.running);
      } else {
        setOllamaRunning(false);
      }
    } catch (error) {
      console.error("Failed to fetch Ollama status:", error);
      setOllamaRunning(false);
    }
  };

  const fetchOllamaModels = async () => {
    try {
      const response = await fetch(getApiUrl("/api/ollama/models"));
      if (response.ok) {
        const data: OllamaModelsResponse = await response.json();
        const names = data.models.map((model) => model.name || model.model);
        setOllamaModels(names);
      }
    } catch (error) {
      console.error("Failed to fetch Ollama models:", error);
    }
  };

  const pullOllamaModel = async () => {
    setOllamaLoading(true);
    setOllamaError("");
    setOllamaMessage(
      "Please wait, it may take some time. Come back in a few minutes"
    );
    try {
      const response = await fetch(getApiUrl("/api/ollama/pull"), {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
        },
        body: JSON.stringify({ model: OLLAMA_MODEL_ID }),
      });
      if (response.ok) {
        await fetchOllamaModels();
      } else {
        setOllamaError("Failed to start Ollama model pull.");
      }
    } catch (error) {
      console.error("Failed to pull Ollama model:", error);
      setOllamaError("Failed to start Ollama model pull.");
    } finally {
      setOllamaLoading(false);
    }
  };

  return (
    <div class="card bg-base-100 shadow-xl">
      <div class="card-body">
        <h2 class="card-title">API Keys</h2>

        <div class="space-y-4">
          {/* Ollama */}
          <div class="form-control w-full max-w-md">
            <label class="label">
              <span class="label-text">Ollama</span>
              <span class="label-text-alt">
                {ollamaRunning() === null
                  ? "Checking..."
                  : ollamaRunning()
                    ? "Running"
                    : "Not running"}
              </span>
            </label>
            <div class="text-sm text-gray-500">
              {OLLAMA_MODEL_LABEL}: {hasOllamaModel() ? "Installed" : "Not installed"}
            </div>
            {!hasOllamaModel() && (
              <div class="mt-3">
                <button
                  class="btn btn-outline btn-sm"
                  onClick={pullOllamaModel}
                  disabled={ollamaLoading() || ollamaRunning() === false}
                >
                  {ollamaLoading() ? "Starting..." : `Pull ${OLLAMA_MODEL_LABEL}`}
                </button>
              </div>
            )}
            {ollamaMessage() && (
              <div class="text-sm text-gray-500 mt-3">{ollamaMessage()}</div>
            )}
            {ollamaError() && (
              <div class="text-sm text-red-600 mt-2">{ollamaError()}</div>
            )}
          </div>

          {/* OpenAI API Key */}
          <div class="form-control w-full max-w-md">
            <label class="label">
              <span class="label-text">OpenAI API Key</span>
              <span class="label-text-alt">
                {getOpenaiKey()?.is_configured
                  ? "Configured"
                  : "Not configured"}
              </span>
            </label>
            {getOpenaiKey()?.is_configured && getOpenaiKey()?.key && (
              <div class="text-sm text-gray-500 mb-2">
                Current: {getOpenaiKey()?.key}
              </div>
            )}
            <input
              type="password"
              placeholder="Enter your OpenAI API key"
              class="input input-bordered w-full"
              value={openaiKey()}
              onInput={(e) => setOpenaiKey(e.target.value)}
            />
          </div>

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
