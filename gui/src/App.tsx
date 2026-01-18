import type { JSX } from "solid-js";
import Sidebar from "./components/Sidebar";

function App(props: { children: any }): JSX.Element {
  return (
    <div class="drawer lg:drawer-open">
      {/* Hidden checkbox for drawer toggle */}
      <input id="sidebar-drawer" type="checkbox" class="drawer-toggle" />

      {/* Main content */}
      <div class="drawer-content min-h-screen bg-base-200">
        {/* Burger menu for mobile/small screens */}
        <div class="lg:hidden navbar bg-base-300">
          <div class="flex-none">
            <label for="sidebar-drawer" class="btn btn-square btn-ghost">
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
          <div class="flex-1 px-2 mx-2">Dwata</div>
        </div>

        {/* Page content */}
        {props.children}
      </div>

      {/* Sidebar */}
      <Sidebar />
    </div>
  );
}

export default App;
