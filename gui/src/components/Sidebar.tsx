import { A } from "@solidjs/router";
import {
  HiOutlineBars3,
  HiOutlineCalendar,
  HiOutlineEnvelope,
  HiOutlineFolder,
  HiOutlineClipboardDocumentCheck,
  HiOutlineCog6Tooth,
  HiOutlineCurrencyDollar,
} from "solid-icons/hi";

export default function Sidebar() {
  return (
    <div class="drawer-side lg:h-screen is-drawer-close:overflow-visible">
      <label
        for="sidebar-drawer"
        aria-label="close sidebar"
        class="drawer-overlay"
      ></label>
      <div class="flex min-h-full flex-col items-start bg-base-100 border-r border-base-300 shadow-lg is-drawer-close:w-16 is-drawer-open:w-64">
        {/* Burger Menu on top */}
        <div class="p-4 w-full flex justify-center">
          <label
            for="sidebar-drawer"
            class="btn btn-ghost btn-square drawer-button"
          >
            <HiOutlineBars3 class="w-6 h-6" />
          </label>
        </div>

        {/* Navigation Links */}
        <div class="flex flex-col flex-grow w-full">
          {/* Projects */}
          <div class="px-4 py-2 w-full">
            <A
              href="/projects"
              class="menu-item flex items-center py-2 px-3 rounded transition-colors hover:bg-base-300 is-drawer-close:justify-center is-drawer-open:justify-start"
              activeClass="bg-primary text-primary-content"
            >
              <HiOutlineFolder class="w-5 h-5 flex-shrink-0" />
              <span class="is-drawer-close:hidden ml-3">Projects</span>
            </A>
          </div>

          {/* Tasks */}
          <div class="px-4 py-2 w-full">
            <A
              href="/tasks"
              class="menu-item flex items-center py-2 px-3 rounded transition-colors hover:bg-base-300 is-drawer-close:justify-center is-drawer-open:justify-start"
              activeClass="bg-primary text-primary-content"
            >
              <HiOutlineClipboardDocumentCheck class="w-5 h-5 flex-shrink-0" />
              <span class="is-drawer-close:hidden ml-3">Tasks</span>
            </A>
          </div>

          {/* Emails */}
          <div class="px-4 py-2 w-full">
            <A
              href="/emails"
              class="menu-item flex items-center py-2 px-3 rounded transition-colors hover:bg-base-300 is-drawer-close:justify-center is-drawer-open:justify-start"
              activeClass="bg-primary text-primary-content"
            >
              <HiOutlineEnvelope class="w-5 h-5 flex-shrink-0" />
              <span class="is-drawer-close:hidden ml-3">Emails</span>
            </A>
          </div>

          {/* Calendar */}
          <div class="px-4 py-2 w-full">
            <A
              href="/calendar"
              class="menu-item flex items-center py-2 px-3 rounded transition-colors hover:bg-base-300 is-drawer-close:justify-center is-drawer-open:justify-start"
              activeClass="bg-primary text-primary-content"
            >
              <HiOutlineCalendar class="w-5 h-5 flex-shrink-0" />
              <span class="is-drawer-close:hidden ml-3">Calendar</span>
            </A>
          </div>

          {/* Financial Health */}
          <div class="px-4 py-2 w-full">
            <A
              href="/financial"
              class="menu-item flex items-center py-2 px-3 rounded transition-colors hover:bg-base-300 is-drawer-close:justify-center is-drawer-open:justify-start"
              activeClass="bg-primary text-primary-content"
            >
              <HiOutlineCurrencyDollar class="w-5 h-5 flex-shrink-0" />
              <span class="is-drawer-close:hidden ml-3">Financial</span>
            </A>
          </div>

          {/* Spacer */}
          <div class="flex-grow"></div>

          {/* Settings at bottom */}
          <div class="px-4 py-2 w-full">
            <A
              href="/settings"
              class="menu-item flex items-center py-2 px-3 rounded transition-colors hover:bg-base-300 is-drawer-close:justify-center is-drawer-open:justify-start"
              activeClass="bg-primary text-primary-content"
            >
              <HiOutlineCog6Tooth class="w-5 h-5 flex-shrink-0" />
              <span class="is-drawer-close:hidden ml-3">Settings</span>
            </A>
          </div>
        </div>
      </div>
    </div>
  );
}
