import { Link, useRouterState } from "@tanstack/react-router";
import {
  Home,
  FolderGit2,
  Bot,
  Settings,
  Server,
} from "lucide-react";

interface NavItem {
  to: string;
  label: string;
  icon: React.ReactNode;
}

const navItems: NavItem[] = [
  { to: "/", label: "Dashboard", icon: <Home className="w-5 h-5" /> },
  {
    to: "/repositories",
    label: "Repositories",
    icon: <FolderGit2 className="w-5 h-5" />,
  },
  { to: "/jobs", label: "Agent Jobs", icon: <Bot className="w-5 h-5" /> },
  { to: "/settings", label: "Settings", icon: <Settings className="w-5 h-5" /> },
];

export function Sidebar() {
  const routerState = useRouterState();
  const currentPath = routerState.location.pathname;

  return (
    <aside className="w-64 min-h-screen bg-slate-900 text-slate-100 flex flex-col">
      {/* Logo / Title */}
      <div className="p-4 border-b border-slate-700">
        <div className="flex items-center gap-2">
          <Server className="w-6 h-6 text-blue-400" />
          <span className="font-bold text-lg">Local Code Agent</span>
        </div>
      </div>

      {/* Navigation */}
      <nav className="flex-1 p-4">
        <ul className="space-y-2">
          {navItems.map((item) => {
            const isActive =
              item.to === "/"
                ? currentPath === "/"
                : currentPath.startsWith(item.to);
            return (
              <li key={item.to}>
                <Link
                  to={item.to}
                  className={`flex items-center gap-3 px-3 py-2 rounded-lg transition-colors ${
                    isActive
                      ? "bg-blue-600 text-white"
                      : "text-slate-300 hover:bg-slate-800 hover:text-white"
                  }`}
                >
                  {item.icon}
                  <span>{item.label}</span>
                </Link>
              </li>
            );
          })}
        </ul>
      </nav>

      {/* Footer */}
      <div className="p-4 border-t border-slate-700 text-sm text-slate-400">
        <p>v0.1.0</p>
      </div>
    </aside>
  );
}
