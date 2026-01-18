import { render } from "solid-js/web";
import { Router, Route } from "@solidjs/router";
import "./index.css";

import App from "./App";
import Projects from "./pages/Projects";
import Settings from "./pages/Settings";

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
      <Route path="/settings" component={Settings} />
    </Router>
  ),
  document.getElementById("root") as HTMLElement,
);
