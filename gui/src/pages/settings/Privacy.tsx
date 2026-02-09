import { usePrivacyMode } from "../../contexts/PrivacyMode";

export default function SettingsPrivacy() {
  const { isEnabled, setEnabled } = usePrivacyMode();

  return (
    <div class="card bg-base-100 shadow-xl">
      <div class="card-body">
        <h2 class="card-title">Screenshot Privacy</h2>
        <p class="text-sm text-base-content/60">
          Blur sensitive text in the UI to keep screenshots safe to share.
        </p>

        <div class="form-control mt-4">
          <label class="label cursor-pointer justify-start gap-4">
            <input
              type="checkbox"
              class="checkbox checkbox-primary"
              checked={isEnabled()}
              onChange={(e) => setEnabled(e.currentTarget.checked)}
            />
            <div>
              <span class="label-text font-medium">Enable privacy blur</span>
              <span class="label-text-alt block text-xs">
                Applies to email subjects, previews, and financial patterns
              </span>
            </div>
          </label>
        </div>
      </div>
    </div>
  );
}
