import {
  HiOutlinePlus,
  HiOutlineCalendar,
  HiOutlineCheckCircle,
  HiOutlineFlag,
  HiOutlineClock,
  HiOutlineClipboardDocumentCheck,
  HiOutlineEllipsisHorizontal,
  HiOutlineFunnel,
} from "solid-icons/hi";
import type { Task } from "../api-types/types";

export default function Tasks() {
  const tasks: Task[] = [
    {
      id: 1,
      project_id: 1,
      title: "Design homepage mockup",
      description:
        "Create wireframes and high-fidelity mockups for the new homepage",
      status: "in-progress",
      priority: "high",
      due_date: "2026-01-25",
      assigned_to: null,
      created_at: BigInt(Date.now()),
      updated_at: BigInt(Date.now()),
    },
    {
      id: 2,
      project_id: 1,
      title: "Set up CI/CD pipeline",
      description:
        "Configure GitHub Actions for automated testing and deployment",
      status: "todo",
      priority: "medium",
      due_date: "2026-02-01",
      assigned_to: null,
      created_at: BigInt(Date.now()),
      updated_at: BigInt(Date.now()),
    },
    {
      id: 3,
      project_id: 2,
      title: "User authentication flow",
      description: "Implement login, signup, and password reset functionality",
      status: "review",
      priority: "critical",
      due_date: "2026-01-22",
      assigned_to: null,
      created_at: BigInt(Date.now()),
      updated_at: BigInt(Date.now()),
    },
    {
      id: 4,
      project_id: 1,
      title: "Write API documentation",
      description: "Document all REST endpoints with examples",
      status: "todo",
      priority: "low",
      due_date: "2026-02-15",
      assigned_to: null,
      created_at: BigInt(Date.now()),
      updated_at: BigInt(Date.now()),
    },
    {
      id: 5,
      project_id: 2,
      title: "Push notification system",
      description:
        "Integrate Firebase Cloud Messaging for real-time notifications",
      status: "done",
      priority: "high",
      due_date: "2026-01-18",
      assigned_to: null,
      created_at: BigInt(Date.now()),
      updated_at: BigInt(Date.now()),
    },
    {
      id: 6,
      project_id: 3,
      title: "Social media graphics",
      description: "Create promotional graphics for Q1 marketing campaign",
      status: "in-progress",
      priority: "medium",
      due_date: "2026-01-28",
      assigned_to: null,
      created_at: BigInt(Date.now()),
      updated_at: BigInt(Date.now()),
    },
  ];

  const statusConfig = {
    todo: {
      label: "To Do",
      badgeClass: "badge-ghost",
      icon: HiOutlineClock,
    },
    "in-progress": {
      label: "In Progress",
      badgeClass: "badge-info",
      icon: HiOutlineCheckCircle,
    },
    review: {
      label: "Review",
      badgeClass: "badge-warning",
      icon: HiOutlineClipboardDocumentCheck,
    },
    done: {
      label: "Done",
      badgeClass: "badge-success",
      icon: HiOutlineCheckCircle,
    },
    cancelled: {
      label: "Cancelled",
      badgeClass: "badge-error",
      icon: HiOutlineCheckCircle,
    },
  };

  const priorityConfig = {
    low: { label: "Low", badgeClass: "badge-ghost" },
    medium: { label: "Medium", badgeClass: "badge-secondary" },
    high: { label: "High", badgeClass: "badge-warning" },
    critical: { label: "Critical", badgeClass: "badge-error" },
  };

  return (
    <div class="p-4 md:p-8">
      <div class="flex flex-col md:flex-row md:items-center md:justify-between mb-8">
        <div>
          <h1 class="text-3xl font-bold mb-2">Tasks</h1>
          <p class="text-base-content/60">
            Track and manage your tasks across all projects
          </p>
        </div>
        <div class="flex gap-2 mt-4 md:mt-0">
          <button class="btn btn-ghost gap-2">
            <HiOutlineFunnel class="w-5 h-5" />
            Filter
          </button>
          <button class="btn btn-primary gap-2">
            <HiOutlinePlus class="w-5 h-5" />
            New Task
          </button>
        </div>
      </div>

      <div class="stats stats-vertical lg:stats-horizontal shadow mb-8 w-full">
        <div class="stat">
          <div class="stat-figure text-primary">
            <HiOutlineClipboardDocumentCheck class="w-8 h-8" />
          </div>
          <div class="stat-title">Total Tasks</div>
          <div class="stat-value text-primary">6</div>
          <div class="stat-desc">Across 3 projects</div>
        </div>

        <div class="stat">
          <div class="stat-figure text-secondary">
            <HiOutlineClock class="w-8 h-8" />
          </div>
          <div class="stat-title">In Progress</div>
          <div class="stat-value text-secondary">2</div>
          <div class="stat-desc">Active work items</div>
        </div>

        <div class="stat">
          <div class="stat-figure text-warning">
            <HiOutlineFlag class="w-8 h-8" />
          </div>
          <div class="stat-title">High Priority</div>
          <div class="stat-value text-warning">3</div>
          <div class="stat-desc">Requiring attention</div>
        </div>

        <div class="stat">
          <div class="stat-figure text-success">
            <HiOutlineCheckCircle class="w-8 h-8" />
          </div>
          <div class="stat-title">Completed</div>
          <div class="stat-value text-success">1</div>
          <div class="stat-desc">This week</div>
        </div>
      </div>

      <div class="overflow-x-auto">
        <table class="table table-zebra">
          <thead>
            <tr>
              <th>Task</th>
              <th>Project</th>
              <th>Status</th>
              <th>Priority</th>
              <th>Due Date</th>
              <th></th>
            </tr>
          </thead>
          <tbody>
            {tasks.map((task) => {
              const status =
                statusConfig[task.status as keyof typeof statusConfig];
              const priority =
                priorityConfig[task.priority as keyof typeof priorityConfig];
              const StatusIcon = status.icon;

              return (
                <tr>
                  <td>
                    <div>
                      <div class="font-bold">{task.title}</div>
                      <div class="text-sm opacity-60">{task.description}</div>
                    </div>
                  </td>
                  <td>
                    {task.project_id && (
                      <span class="badge badge-sm badge-outline">
                        Project {task.project_id}
                      </span>
                    )}
                  </td>
                  <td>
                    <span class={`badge badge-sm ${status.badgeClass} gap-1`}>
                      <StatusIcon class="w-3 h-3" />
                      {status.label}
                    </span>
                  </td>
                  <td>
                    <span class={`badge badge-sm ${priority.badgeClass}`}>
                      {priority.label}
                    </span>
                  </td>
                  <td>
                    {task.due_date && (
                      <div class="flex items-center gap-1 text-sm">
                        <HiOutlineCalendar class="w-4 h-4" />
                        {task.due_date}
                      </div>
                    )}
                  </td>
                  <td>
                    <div class="dropdown dropdown-end">
                      <button class="btn btn-ghost btn-sm btn-circle">
                        <HiOutlineEllipsisHorizontal class="w-5 h-5" />
                      </button>
                      <ul class="dropdown-content z-[1] menu p-2 shadow bg-base-100 rounded-box w-32">
                        <li>
                          <a>Edit</a>
                        </li>
                        <li>
                          <a class="text-error">Delete</a>
                        </li>
                      </ul>
                    </div>
                  </td>
                </tr>
              );
            })}
          </tbody>
        </table>
      </div>

      {tasks.length === 0 && (
        <div class="flex flex-col items-center justify-center py-16 px-4">
          <div class="text-center max-w-md">
            <HiOutlineClipboardDocumentCheck class="w-16 h-16 mx-auto text-base-content/30 mb-4" />
            <h3 class="text-xl font-semibold mb-2">No tasks yet</h3>
            <p class="text-base-content/60 mb-6">
              Create your first task to start tracking your work. Organize tasks
              by project, set priorities, and monitor progress.
            </p>
            <button class="btn btn-primary gap-2">
              <HiOutlinePlus class="w-5 h-5" />
              Create Your First Task
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
