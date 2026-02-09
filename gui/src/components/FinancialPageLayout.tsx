import { Show, type JSX, type ParentProps } from "solid-js";

type FinancialPageLayoutProps = ParentProps<{
  title: string;
  subtitle?: string;
  footer?: JSX.Element;
}>;

export default function FinancialPageLayout(props: FinancialPageLayoutProps) {
  return (
    <div class="h-full min-h-0 flex flex-col overflow-hidden">
      <header class="pt-4 px-4 md:pt-8 md:px-8 mb-6">
        <h1 class="text-3xl font-bold mb-2">{props.title}</h1>
        <Show when={props.subtitle}>
          <p class="text-base-content/60">{props.subtitle}</p>
        </Show>
      </header>

      <main class="flex-1 min-h-0 overflow-y-auto px-4 md:px-8">{props.children}</main>

      <Show when={props.footer}>
        <footer class="sticky bottom-0 mt-auto border-t border-base-300 bg-base-100/95 backdrop-blur">
          <div class="flex items-center justify-end gap-2 px-4 py-3 md:px-8">
            {props.footer}
          </div>
        </footer>
      </Show>
    </div>
  );
}
