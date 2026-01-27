import {
  HiOutlinePlus,
  HiOutlineCalendar,
  HiOutlineCheckCircle,
  HiOutlineCurrencyDollar,
  HiOutlineBell,
  HiOutlineClock,
  HiOutlineUsers,
} from "solid-icons/hi";
import type { Project } from "../api-types/types";

export default function Projects() {
  // Mock data for demonstration
  const projects: Project[] = [
    {
      id: 1,
      name: "Website Redesign",
      description:
        "Complete overhaul of the company website with modern design",
      status: "active",
      tasks_completed: 12,
      tasks_total: 24,
      deadline: "2026-02-15",
      notifications: 3,
      created_at: BigInt(Date.now()),
      updated_at: BigInt(Date.now()),
    },
    {
      id: 2,
      name: "Mobile App Development",
      description: "Native iOS and Android app for customer engagement",
      status: "planning",
      tasks_completed: 3,
      tasks_total: 45,
      deadline: "2026-04-30",
      notifications: 1,
      created_at: BigInt(Date.now()),
      updated_at: BigInt(Date.now()),
    },
    {
      id: 3,
      name: "Marketing Campaign Q1",
      description: "Digital marketing campaign for product launch",
      status: "on-hold",
      tasks_completed: 8,
      tasks_total: 15,
      deadline: "2026-03-31",
      notifications: 0,
      created_at: BigInt(Date.now()),
      updated_at: BigInt(Date.now()),
    },
  ];

  return (
    <div class="p-4 md:p-8">
      {/* Header */}
      <div class="flex flex-col md:flex-row md:items-center md:justify-between mb-8">
        <div>
          <h1 class="text-3xl font-bold mb-2">Projects</h1>
          <p class="text-base-content/60">
            Manage your work and hobby projects in one place
          </p>
        </div>
        <button class="btn btn-primary mt-4 md:mt-0 gap-2">
          <HiOutlinePlus class="w-5 h-5" />
          New Project
        </button>
      </div>

      {/* Stats Overview */}
      <div class="stats stats-vertical lg:stats-horizontal shadow mb-8 w-full">
        <div class="stat">
          <div class="stat-figure text-primary">
            <HiOutlineCheckCircle class="w-8 h-8" />
          </div>
          <div class="stat-title">Active Projects</div>
          <div class="stat-value text-primary">3</div>
          <div class="stat-desc">2 completed this month</div>
        </div>

        <div class="stat">
          <div class="stat-figure text-secondary">
            <HiOutlineClock class="w-8 h-8" />
          </div>
          <div class="stat-title">Active Tasks</div>
          <div class="stat-value text-secondary">23</div>
          <div class="stat-desc">56 total tasks across all projects</div>
        </div>

        <div class="stat">
          <div class="stat-figure text-accent">
            <HiOutlineCurrencyDollar class="w-8 h-8" />
          </div>
          <div class="stat-title">Pending Invoices</div>
          <div class="stat-value text-accent">$4,200</div>
          <div class="stat-desc">2 invoices awaiting payment</div>
        </div>

        <div class="stat">
          <div class="stat-figure text-warning">
            <HiOutlineBell class="w-8 h-8" />
          </div>
          <div class="stat-title">Notifications</div>
          <div class="stat-value text-warning">4</div>
          <div class="stat-desc">Requires your attention</div>
        </div>
      </div>

      {/* Projects Grid */}
      <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-6">
        {projects.map((project) => (
          <div class="card bg-base-100 shadow-sm border border-base-300 hover:shadow-md transition-shadow">
            <div class="card-body">
              {/* Project Header */}
              <div class="flex items-start justify-between">
                <h2 class="card-title text-lg">{project.name}</h2>
                <span
                  class="badge badge-sm"
                  classList={{
                    "badge-primary": project.status === "active",
                    "badge-info": project.status === "planning",
                    "badge-warning": project.status === "on-hold",
                    "badge-success": project.status === "completed",
                    "badge-ghost": project.status === "archived",
                  }}
                >
                  {project.status}
                </span>
              </div>

              {/* Project Description */}
              <p class="text-sm text-base-content/70 line-clamp-2">
                {project.description}
              </p>

              {/* Project Stats */}
              <div class="flex flex-col gap-3 mt-4">
                {/* Progress */}
                <div>
                  <div class="flex justify-between text-xs mb-1">
                    <span class="text-base-content/60">Tasks Progress</span>
                    <span class="font-medium">
                      {project.tasks_completed}/{project.tasks_total}
                    </span>
                  </div>
                  <progress
                    class="progress progress-primary w-full"
                    value={project.tasks_completed}
                    max={project.tasks_total}
                  ></progress>
                </div>

                {/* Meta Info */}
                <div class="flex flex-wrap gap-3 text-xs text-base-content/60">
                  {project.deadline && (
                    <div class="flex items-center gap-1">
                      <HiOutlineCalendar class="w-4 h-4" />
                      <span>Due {project.deadline}</span>
                    </div>
                  )}
                  {project.notifications > 0 && (
                    <div class="flex items-center gap-1">
                      <HiOutlineBell class="w-4 h-4" />
                      <span>{project.notifications} updates</span>
                    </div>
                  )}
                  <div class="flex items-center gap-1">
                    <HiOutlineUsers class="w-4 h-4" />
                    <span>3 members</span>
                  </div>
                </div>
              </div>

              {/* Card Actions */}
              <div class="card-actions justify-end mt-4">
                <button class="btn btn-ghost btn-sm">View Details</button>
                <button class="btn btn-primary btn-sm">Open</button>
              </div>
            </div>
          </div>
        ))}
      </div>

      {/* Empty State (when no projects) */}
      {projects.length === 0 && (
        <div class="flex flex-col items-center justify-center py-16 px-4">
          <div class="text-center max-w-md">
            <HiOutlineCheckCircle class="w-16 h-16 mx-auto text-base-content/30 mb-4" />
            <h3 class="text-xl font-semibold mb-2">No projects yet</h3>
            <p class="text-base-content/60 mb-6">
              Get started by creating your first project. Track tasks,
              deadlines, invoices, and collaborate with your team.
            </p>
            <button class="btn btn-primary gap-2">
              <HiOutlinePlus class="w-5 h-5" />
              Create Your First Project
            </button>
          </div>
        </div>
      )}
    </div>
  );
}
