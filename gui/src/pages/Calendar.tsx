import {
  HiOutlinePlus,
  HiOutlineCalendar,
  HiOutlineChevronLeft,
  HiOutlineChevronRight,
  HiOutlineUserGroup,
  HiOutlineFlag,
} from "solid-icons/hi";
import { createMemo, createSignal, For } from "solid-js";
import type { Event } from "../api-types/types";

const monthNames = [
  "January",
  "February",
  "March",
  "April",
  "May",
  "June",
  "July",
  "August",
  "September",
  "October",
  "November",
  "December",
];

const dayNames = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

const getDaysInMonth = (year: number, month: number): number => {
  return new Date(year, month + 1, 0).getDate();
};

const getFirstDayOfMonth = (year: number, month: number): number => {
  return new Date(year, month, 1).getDay();
};

export default function Calendar() {
  const [currentDate, setCurrentDate] = createSignal(new Date());
  const [selectedDate, setSelectedDate] = createSignal<string | null>(null);

  const events: Event[] = [
    {
      id: 1,
      name: "Project Kickoff Meeting",
      description: "Initial planning and requirements gathering",
      project_id: 1,
      task_id: null,
      date: "2026-01-22",
      attendee_user_ids: null,
      created_at: BigInt(Date.now()),
      updated_at: BigInt(Date.now()),
    },
    {
      id: 2,
      name: "Design Review",
      description: "Review homepage mockups with stakeholders",
      project_id: 1,
      task_id: 1,
      date: "2026-01-23",
      attendee_user_ids: null,
      created_at: BigInt(Date.now()),
      updated_at: BigInt(Date.now()),
    },
    {
      id: 3,
      name: "Sprint Planning",
      description: "Plan tasks for next two weeks",
      project_id: 2,
      task_id: null,
      date: "2026-01-25",
      attendee_user_ids: null,
      created_at: BigInt(Date.now()),
      updated_at: BigInt(Date.now()),
    },
    {
      id: 4,
      name: "Client Demo",
      description: "Showcase mobile app progress",
      project_id: 2,
      task_id: null,
      date: "2026-01-27",
      attendee_user_ids: null,
      created_at: BigInt(Date.now()),
      updated_at: BigInt(Date.now()),
    },
    {
      id: 5,
      name: "Marketing Strategy Session",
      description: "Plan Q1 campaign execution",
      project_id: 3,
      task_id: 6,
      date: "2026-01-28",
      attendee_user_ids: null,
      created_at: BigInt(Date.now()),
      updated_at: BigInt(Date.now()),
    },
    {
      id: 6,
      name: "Team Retrospective",
      description: "Review past sprint and improvements",
      project_id: null,
      task_id: null,
      date: "2026-01-30",
      attendee_user_ids: null,
      created_at: BigInt(Date.now()),
      updated_at: BigInt(Date.now()),
    },
    {
      id: 7,
      name: "API Integration Workshop",
      description: "Hands-on session with backend team",
      project_id: 2,
      task_id: 2,
      date: "2026-02-03",
      attendee_user_ids: null,
      created_at: BigInt(Date.now()),
      updated_at: BigInt(Date.now()),
    },
    {
      id: 8,
      name: "Quarterly Review",
      description: "Review performance and set goals",
      project_id: null,
      task_id: null,
      date: "2026-02-05",
      attendee_user_ids: null,
      created_at: BigInt(Date.now()),
      updated_at: BigInt(Date.now()),
    },
  ];

  const year = createMemo(() => currentDate().getFullYear());
  const month = createMemo(() => currentDate().getMonth());

  const daysInMonth = createMemo(() => getDaysInMonth(year(), month()));
  const firstDayOfMonth = createMemo(() => getFirstDayOfMonth(year(), month()));

  const prevMonth = () => {
    const date = new Date(currentDate());
    date.setMonth(date.getMonth() - 1);
    setCurrentDate(date);
  };

  const nextMonth = () => {
    const date = new Date(currentDate());
    date.setMonth(date.getMonth() + 1);
    setCurrentDate(date);
  };

  const goToToday = () => {
    setCurrentDate(new Date());
  };

  const getEventsForDate = (dateStr: string) => {
    return events.filter((event) => event.date === dateStr);
  };

  const calendarDays = createMemo(() => {
    const days: Array<{
      day: number | null;
      dateStr: string;
      isCurrentMonth: boolean;
      isToday: boolean;
    }> = [];

    const today = new Date();
    const isCurrentMonth =
      today.getFullYear() === year() && today.getMonth() === month();

    for (let i = 0; i < firstDayOfMonth(); i++) {
      const prevMonthDate = new Date(
        year(),
        month(),
        0 - firstDayOfMonth() + i + 1,
      );
      days.push({
        day: prevMonthDate.getDate(),
        dateStr: prevMonthDate.toISOString().split("T")[0],
        isCurrentMonth: false,
        isToday: false,
      });
    }

    for (let day = 1; day <= daysInMonth(); day++) {
      const dateStr = `${year()}-${String(month() + 1).padStart(2, "0")}-${String(
        day,
      ).padStart(2, "0")}`;
      const isToday = isCurrentMonth && day === today.getDate();
      days.push({
        day,
        dateStr,
        isCurrentMonth: true,
        isToday,
      });
    }

    const totalCells = Math.ceil(days.length / 7) * 7;
    for (let i = days.length; i < totalCells; i++) {
      const nextMonthDate = new Date(year(), month() + 1, i - days.length + 1);
      days.push({
        day: nextMonthDate.getDate(),
        dateStr: nextMonthDate.toISOString().split("T")[0],
        isCurrentMonth: false,
        isToday: false,
      });
    }

    return days;
  });

  const selectedDateEvents = createMemo(() => {
    if (selectedDate()) {
      return getEventsForDate(selectedDate()!);
    }
    return [];
  });

  const todayEvents = createMemo(() => {
    const today = new Date().toISOString().split("T")[0];
    return getEventsForDate(today);
  });

  return (
    <div class="p-4 md:p-8">
      <div class="flex flex-col lg:flex-row gap-6">
        <div class="flex-1">
          <div class="flex flex-col md:flex-row md:items-center md:justify-between mb-6">
            <div>
              <h1 class="text-3xl font-bold mb-2">Calendar</h1>
              <p class="text-base-content/60">
                Manage your events, meetings, and deadlines
              </p>
            </div>
            <div class="flex gap-2 mt-4 md:mt-0">
              <button class="btn btn-ghost btn-sm" onClick={goToToday}>
                Today
              </button>
              <button class="btn btn-ghost btn-circle" onClick={prevMonth}>
                <HiOutlineChevronLeft class="w-5 h-5" />
              </button>
              <div class="btn btn-ghost btn-disabled">
                {monthNames[month()]} {year()}
              </div>
              <button class="btn btn-ghost btn-circle" onClick={nextMonth}>
                <HiOutlineChevronRight class="w-5 h-5" />
              </button>
              <button class="btn btn-primary gap-2">
                <HiOutlinePlus class="w-5 h-5" />
                New Event
              </button>
            </div>
          </div>

          <div class="stats stats-vertical lg:stats-horizontal shadow mb-6 w-full">
            <div class="stat">
              <div class="stat-figure text-primary">
                <HiOutlineCalendar class="w-8 h-8" />
              </div>
              <div class="stat-title">Events This Month</div>
              <div class="stat-value text-primary">
                {
                  events.filter((e) => {
                    const d = new Date(e.date);
                    return (
                      d.getMonth() === month() && d.getFullYear() === year()
                    );
                  }).length
                }
              </div>
              <div class="stat-desc">Across all projects</div>
            </div>

            <div class="stat">
              <div class="stat-figure text-secondary">
                <HiOutlineUserGroup class="w-8 h-8" />
              </div>
              <div class="stat-title">Today's Events</div>
              <div class="stat-value text-secondary">
                {todayEvents().length}
              </div>
              <div class="stat-desc">Scheduled for today</div>
            </div>

            <div class="stat">
              <div class="stat-figure text-warning">
                <HiOutlineFlag class="w-8 h-8" />
              </div>
              <div class="stat-title">Upcoming</div>
              <div class="stat-value text-warning">
                {events.filter((e) => new Date(e.date) >= new Date()).length}
              </div>
              <div class="stat-desc">Events remaining</div>
            </div>
          </div>

          <div class="bg-base-100 rounded-lg shadow overflow-hidden border border-base-300">
            <div class="grid grid-cols-7 bg-base-200 border-b border-base-300">
              <For each={dayNames}>
                {(day) => (
                  <div class="p-3 text-center text-sm font-semibold text-base-content/70">
                    {day}
                  </div>
                )}
              </For>
            </div>

            <div class="grid grid-cols-7 auto-rows-fr">
              <For each={calendarDays()}>
                {(day) => (
                  <div
                    class={`min-h-24 p-2 border-t border-r border-base-300 cursor-pointer transition-colors hover:bg-base-200 ${day.isToday ? "bg-primary/5" : ""} ${!day.isCurrentMonth ? "bg-base-100/50" : ""}`}
                    onClick={() => setSelectedDate(day.dateStr)}
                  >
                    <div
                      class={`text-sm font-medium ${
                        day.isToday
                          ? "text-primary bg-primary text-primary-content w-7 h-7 rounded-full flex items-center justify-center"
                          : day.isCurrentMonth
                            ? "text-base-content"
                            : "text-base-content/40"
                      }`}
                    >
                      {day.day}
                    </div>
                    <div class="mt-1 space-y-1">
                      <For each={getEventsForDate(day.dateStr)}>
                        {(event) => (
                          <div
                            class={`text-xs p-1 rounded truncate cursor-pointer hover:opacity-80 ${event.project_id ? "badge badge-primary badge-sm" : "badge badge-secondary badge-sm"}`}
                          >
                            {event.name}
                          </div>
                        )}
                      </For>
                    </div>
                  </div>
                )}
              </For>
            </div>
          </div>
        </div>

        <div class="lg:w-96">
          <div class="sticky top-4">
            <div class="bg-base-100 rounded-lg shadow border border-base-300">
              <div class="p-4 border-b border-base-300">
                <h2 class="font-semibold text-lg">
                  {selectedDate()
                    ? new Date(selectedDate()).toLocaleDateString("en-US", {
                        weekday: "long",
                        month: "long",
                        day: "numeric",
                      })
                    : "Today's Events"}
                </h2>
              </div>

              <div class="p-4">
                <For
                  each={selectedDate() ? selectedDateEvents() : todayEvents()}
                >
                  {(event) => (
                    <div class="mb-4 last:mb-0">
                      <div class="flex items-start gap-3">
                        <div
                          class={`mt-1 w-3 h-3 rounded-full flex-shrink-0 ${
                            event.project_id ? "bg-primary" : "bg-secondary"
                          }`}
                        ></div>
                        <div class="flex-1 min-w-0">
                          <div class="font-medium text-sm">{event.name}</div>
                          {event.description && (
                            <div class="text-xs text-base-content/60 mt-1">
                              {event.description}
                            </div>
                          )}
                          <div class="flex gap-2 mt-2">
                            {event.project_id && (
                              <span class="badge badge-xs badge-outline">
                                Project {event.project_id}
                              </span>
                            )}
                            {event.task_id && (
                              <span class="badge badge-xs badge-ghost">
                                Task {event.task_id}
                              </span>
                            )}
                          </div>
                        </div>
                      </div>
                    </div>
                  )}
                </For>

                {(selectedDate() ? selectedDateEvents() : todayEvents())
                  .length === 0 && (
                  <div class="text-center py-8 text-base-content/60">
                    <HiOutlineCalendar class="w-12 h-12 mx-auto mb-3 opacity-30" />
                    <p class="text-sm">No events scheduled</p>
                  </div>
                )}
              </div>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
