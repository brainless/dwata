import { Show, type JSX, type ParentProps } from "solid-js";

type EmailPageLayoutProps = ParentProps<{
  header?: JSX.Element;
  footer?: JSX.Element;
}>;

export default function EmailPageLayout(props: EmailPageLayoutProps) {
  return (
    <div class="h-full min-h-full flex flex-col bg-base-200">
      <Show when={props.header}>
        <header class="shrink-0">{props.header}</header>
      </Show>
      <main class="flex-1 min-h-0">{props.children}</main>
      <Show when={props.footer}>
        <footer class="shrink-0 border-t border-base-300 bg-base-100">
          {props.footer}
        </footer>
      </Show>
    </div>
  );
}
