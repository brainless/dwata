import { A } from "@solidjs/router";

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
            <svg
              xmlns="http://www.w3.org/2000/svg"
              fill="none"
              viewBox="0 0 24 24"
              stroke-width="1.5"
              stroke="currentColor"
              class="w-6 h-6"
            >
              <path
                stroke-linecap="round"
                stroke-linejoin="round"
                d="M3.75 6.75h16.5M3.75 12h16.5m-16.5 5.25h16.5"
              />
            </svg>
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
              <svg
                xmlns="http://www.w3.org/2000/svg"
                fill="none"
                viewBox="0 0 24 24"
                stroke-width="1.5"
                stroke="currentColor"
                class="w-5 h-5 flex-shrink-0"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  d="M19.5 14.25v-2.625a3.375 3.375 0 0 0-3.375-3.375h-1.5A1.125 1.125 0 0 1 13.5 7.125v-1.5a3.375 3.375 0 0 0-3.375-3.375H8.25m0 12.75h7.5m-7.5 3H12M10.5 2.25H5.625c-.621 0-1.125.504-1.125 1.125v17.25c0 .621.504 1.125 1.125 1.125h12.75c.621 0 1.125-.504 1.125-1.125V11.25a9 9 0 0 0-9-9Z"
                />
              </svg>
              <span class="is-drawer-close:hidden ml-3">Projects</span>
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
              <svg
                xmlns="http://www.w3.org/2000/svg"
                fill="none"
                viewBox="0 0 24 24"
                stroke-width="1.5"
                stroke="currentColor"
                class="w-5 h-5 flex-shrink-0"
              >
                <path
                  stroke-linecap="round"
                  stroke-linejoin="round"
                  d="M4.5 12a7.5 7.5 0 0015 0m-15 0a7.5 7.5 0 1115 0m-15 0c0 1.657 4.03 3 9 3s9-1.343 9-3m-9 3c-1.657 0-3-4.03-3-9s1.343-9 3-9m0 18c1.657 0 3-4.03 3-9s-1.343-9-3-9m-9 9a9 9 0 019-9"
                />
              </svg>
              <span class="is-drawer-close:hidden ml-3">Settings</span>
            </A>
          </div>
        </div>
      </div>
    </div>
  );
}
