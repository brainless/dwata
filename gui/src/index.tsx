import { render } from "solid-js/web";
import { Router, Route } from "@solidjs/router";
import "./index.css";

import App from "./App";
import BackgroundJobs from "./pages/BackgroundJobs";
import Calendar from "./pages/Calendar";
import Emails from "./pages/Emails";
import FinancialHealth from "./pages/FinancialHealth";
import Projects from "./pages/Projects";
import Settings from "./pages/Settings";
import Tasks from "./pages/Tasks";

render(
  () => (
    <Router root={App}>
      <Route
        path="/"
        component={() => (
          <div class="flex items-center justify-center h-screen">
            <h1 class="text-4xl font-bold">Welcome to Dwata</h1>
          </div>
        )}
      />
      <Route path="/projects" component={Projects} />
      <Route path="/tasks" component={Tasks} />
      <Route path="/emails" component={Emails} />
      <Route path="/emails/account/:accountId" component={Emails} />
      <Route path="/emails/account/:accountId/folder/:folderId" component={Emails} />
      <Route path="/emails/account/:accountId/label/:labelId" component={Emails} />
      <Route path="/calendar" component={Calendar} />
      <Route path="/financial" component={FinancialHealth} />
      <Route path="/jobs" component={BackgroundJobs} />
      <Route path="/settings" component={Settings} />
      <Route path="/settings/:tab" component={Settings} />
    </Router>
  ),
  document.getElementById("root") as HTMLElement,
);
