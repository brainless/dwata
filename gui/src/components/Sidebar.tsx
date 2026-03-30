import { A } from "@solidjs/router";
import {
  HiOutlineBars3,
  HiOutlineEnvelope,
  HiOutlineCog6Tooth,
  HiOutlineCurrencyDollar,
  HiOutlineCpuChip,
  HiOutlineDocumentText,
  HiOutlineShoppingBag,
  HiOutlineBuildingOffice2,
  HiOutlineUser,
  HiOutlineMapPin,
  HiOutlineArrowPath,
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

          {/* Financial */}
          <div class="px-4 py-2 w-full">
            <A
              href="/financial/transactions"
              class="menu-item flex items-center py-2 px-3 rounded transition-colors hover:bg-base-300 is-drawer-close:justify-center is-drawer-open:justify-start"
              activeClass="bg-primary text-primary-content"
            >
              <HiOutlineCurrencyDollar class="w-5 h-5 flex-shrink-0" />
              <span class="is-drawer-close:hidden ml-3">Transactions</span>
            </A>
          </div>
          <div class="px-4 py-2 w-full">
            <A
              href="/financial/bills"
              class="menu-item flex items-center py-2 px-3 rounded transition-colors hover:bg-base-300 is-drawer-close:justify-center is-drawer-open:justify-start"
              activeClass="bg-primary text-primary-content"
            >
              <HiOutlineDocumentText class="w-5 h-5 flex-shrink-0" />
              <span class="is-drawer-close:hidden ml-3">Bills</span>
            </A>
          </div>
          <div class="px-4 py-2 w-full">
            <A
              href="/kg/subscriptions"
              class="menu-item flex items-center py-2 px-3 rounded transition-colors hover:bg-base-300 is-drawer-close:justify-center is-drawer-open:justify-start"
              activeClass="bg-primary text-primary-content"
            >
              <HiOutlineArrowPath class="w-5 h-5 flex-shrink-0" />
              <span class="is-drawer-close:hidden ml-3">Subscriptions</span>
            </A>
          </div>
          <div class="px-4 py-2 w-full">
            <A
              href="/kg/orders"
              class="menu-item flex items-center py-2 px-3 rounded transition-colors hover:bg-base-300 is-drawer-close:justify-center is-drawer-open:justify-start"
              activeClass="bg-primary text-primary-content"
            >
              <HiOutlineShoppingBag class="w-5 h-5 flex-shrink-0" />
              <span class="is-drawer-close:hidden ml-3">Orders</span>
            </A>
          </div>
          <div class="px-4 py-2 w-full">
            <A
              href="/kg/organisations"
              class="menu-item flex items-center py-2 px-3 rounded transition-colors hover:bg-base-300 is-drawer-close:justify-center is-drawer-open:justify-start"
              activeClass="bg-primary text-primary-content"
            >
              <HiOutlineBuildingOffice2 class="w-5 h-5 flex-shrink-0" />
              <span class="is-drawer-close:hidden ml-3">Organisations</span>
            </A>
          </div>
          <div class="px-4 py-2 w-full">
            <A
              href="/kg/persons"
              class="menu-item flex items-center py-2 px-3 rounded transition-colors hover:bg-base-300 is-drawer-close:justify-center is-drawer-open:justify-start"
              activeClass="bg-primary text-primary-content"
            >
              <HiOutlineUser class="w-5 h-5 flex-shrink-0" />
              <span class="is-drawer-close:hidden ml-3">People</span>
            </A>
          </div>
          <div class="px-4 py-2 w-full">
            <A
              href="/kg/locations"
              class="menu-item flex items-center py-2 px-3 rounded transition-colors hover:bg-base-300 is-drawer-close:justify-center is-drawer-open:justify-start"
              activeClass="bg-primary text-primary-content"
            >
              <HiOutlineMapPin class="w-5 h-5 flex-shrink-0" />
              <span class="is-drawer-close:hidden ml-3">Locations</span>
            </A>
          </div>

          {/* Spacer */}
          <div class="flex-grow"></div>

          {/* Data Extraction - above Settings */}
          <div class="px-4 py-2 w-full">
            <A
              href="/data-extraction"
              class="menu-item flex items-center py-2 px-3 rounded transition-colors hover:bg-base-300 is-drawer-close:justify-center is-drawer-open:justify-start"
              activeClass="bg-primary text-primary-content"
            >
              <HiOutlineCpuChip class="w-5 h-5 flex-shrink-0" />
              <span class="is-drawer-close:hidden ml-3">Data Extraction</span>
            </A>
          </div>

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
