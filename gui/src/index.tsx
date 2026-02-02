import { render } from "solid-js/web";
import { Router, Route } from "@solidjs/router";
import "./index.css";

import App from "./App";
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
      <Route path="/calendar" component={Calendar} />
      <Route path="/financial" component={FinancialHealth} />
      <Route path="/settings" component={Settings} />
      <Route path="/settings/:tab" component={Settings} />
    </Router>
  ),
  document.getElementById("root") as HTMLElement,
);
