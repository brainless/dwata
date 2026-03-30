import { render } from "solid-js/web";
import { Router, Route } from "@solidjs/router";
import "./index.css";

import App from "./App";
import Calendar from "./pages/Calendar";
import DataExtraction from "./pages/DataExtraction";
import Emails from "./pages/Emails";
import FinancialBills from "./pages/FinancialBills";
import FinancialTransactions from "./pages/FinancialTransactions";
import Locations from "./pages/Locations";
import Orders from "./pages/Orders";
import Organisations from "./pages/Organisations";
import Persons from "./pages/Persons";
import Projects from "./pages/Projects";
import Settings from "./pages/Settings";
import Subscriptions from "./pages/Subscriptions";
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
      <Route path="/financial/transactions" component={FinancialTransactions} />
      <Route path="/financial/bills" component={FinancialBills} />
      <Route path="/kg/subscriptions" component={Subscriptions} />
      <Route path="/kg/orders" component={Orders} />
      <Route path="/kg/organisations" component={Organisations} />
      <Route path="/kg/persons" component={Persons} />
      <Route path="/kg/locations" component={Locations} />
      <Route path="/data-extraction" component={DataExtraction} />
      <Route path="/settings" component={Settings} />
      <Route path="/settings/:tab" component={Settings} />
    </Router>
  ),
  document.getElementById("root") as HTMLElement,
);
