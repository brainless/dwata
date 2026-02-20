import { A } from "@solidjs/router";
import FinancialPageLayout from "../components/FinancialPageLayout";

export default function FinancialTemplates() {
  const footerActions = (
    <>
      <A href="/financial" class="btn btn-ghost btn-sm">
        Back to Financial
      </A>
      <A href="/financial/templates/detect" class="btn btn-secondary btn-sm">
        Detect Templates
      </A>
    </>
  );

  return (
    <FinancialPageLayout
      title="Financial Overview: Templates"
      subtitle="Regex template management has been removed. Template-based extraction UI will be integrated here."
      footer={footerActions}
    >
      <div class="card bg-base-100 shadow">
        <div class="card-body">
          <h2 class="card-title">Legacy Templates Removed</h2>
          <p class="text-sm text-base-content/70">
            The old regex-based financial template system is deprecated and has been removed from
            API wiring.
          </p>
          <p class="text-sm text-base-content/70">
            This page will be replaced by the reverse-template based management view.
          </p>
        </div>
      </div>
    </FinancialPageLayout>
  );
}
