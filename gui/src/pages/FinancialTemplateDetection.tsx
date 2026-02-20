import { A } from "@solidjs/router";
import { HiOutlineSparkles } from "solid-icons/hi";
import FinancialPageLayout from "../components/FinancialPageLayout";

export default function FinancialTemplateDetection() {
  const footerActions = (
    <>
      <A href="/financial/templates" class="btn btn-ghost btn-sm">
        Back to Templates
      </A>
    </>
  );

  return (
    <FinancialPageLayout
      title="Financial Overview: Detect Templates"
      subtitle="Reverse-template detection will be wired here."
      footer={footerActions}
    >
      <div class="card bg-base-100 shadow">
        <div class="card-body">
          <div class="flex items-center gap-3">
            <HiOutlineSparkles class="w-6 h-6 text-primary" />
            <h2 class="card-title">Legacy Detection Removed</h2>
          </div>
          <p class="text-sm text-base-content/70">
            The temporary sender-scan endpoint used by the regex flow has been removed.
          </p>
          <p class="text-sm text-base-content/70">
            This page will run reverse-template detection via the new extractor API.
          </p>
        </div>
      </div>
    </FinancialPageLayout>
  );
}
