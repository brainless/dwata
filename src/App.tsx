import { Component } from "solid-js";
import { RouteSectionProps } from "@solidjs/router";
import NavigationBar from "./widgets/navigation/NavigationBar";
import { UserProvider } from "./stores/user";
import Sidebar from "./widgets/navigation/Sidebar";
import { WorkspaceProvider } from "./stores/workspace";
import { SchemaProvider } from "./stores/schema";
import {
  UserInterfaceProvider,
  useUserInterface,
} from "./stores/userInterface";
import { NextTaskProvider } from "./stores/nextTask";
import { ProcessLogLoader, ProcessLogProvider } from "./stores/processLog";

const MainContent: Component<RouteSectionProps> = (props) => {
  const [_, { getColors }] = useUserInterface();

  return (
    <div
      class="grow p-4"
      style={{
        "background-color": getColors().colors["editor.background"],
      }}
    >
      {props.children}
    </div>
  );
};

const App: Component<RouteSectionProps> = (props) => {
  return (
    <ProcessLogProvider>
      <UserProvider>
        <UserInterfaceProvider>
          <WorkspaceProvider>
            <SchemaProvider>
              <NextTaskProvider>
                <div class="flex flex-col w-full h-full">
                  <ProcessLogLoader />
                  <NavigationBar />

                  <div class="flex bg-gray-300 grow h-full overflow-hidden">
                    <Sidebar />

                    <MainContent {...props} />
                  </div>
                </div>
              </NextTaskProvider>
            </SchemaProvider>
          </WorkspaceProvider>
        </UserInterfaceProvider>
      </UserProvider>
    </ProcessLogProvider>
  );
};

export default App;
