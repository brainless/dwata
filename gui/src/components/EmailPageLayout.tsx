import { Show, type JSX, type ParentProps } from "solid-js";

type EmailPageLayoutProps = ParentProps<{
  header?: JSX.Element;
  footer?: JSX.Element;
}>;

export default function EmailPageLayout(props: EmailPageLayoutProps) {
  return (
    <div class="h-full min-h-0 flex flex-col bg-base-200 overflow-hidden">
      <Show when={props.header}>
        <header class="shrink-0 sticky top-0 z-10">{props.header}</header>
      </Show>
      <main class="flex-1 min-h-0 relative overflow-hidden">
        <div class="h-full min-h-0 pb-14">{props.children}</div>
        <Show when={props.footer}>
          <footer class="absolute bottom-0 left-0 right-0 z-10 border-t border-base-300 bg-base-100/95 backdrop-blur">
            {props.footer}
          </footer>
        </Show>
      </main>
    </div>
  );
}
