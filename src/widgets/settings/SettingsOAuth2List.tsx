import { Component, For, createMemo } from "solid-js";
import { useWorkspace } from "../../stores/workspace";
import { useUserInterface } from "../../stores/userInterface";
import { useNavigate } from "@solidjs/router";
import { OAuth2 } from "../../api_types/OAuth2";
import { Module } from "../../api_types/Module";

const SettingsOAuth2Item: Component<OAuth2> = (props) => {
  const [_, { getColors }] = useUserInterface();
  const navigate = useNavigate();

  const handleClick = () => {
    navigate(`/settings/oauth2/edit/${props.id}`);
  };

  return (
    <div
      class="p-4 text-white rounded-md border cursor-pointer"
      style={{
        "background-color": getColors().colors["panel.background"],
        "border-color": getColors().colors["panel.border"],
      }}
      onClick={handleClick}
    >
      <i class="fa-solid fa-database w-6 text-gray-500" />
      {props.handle}
      <div>
        <span class="text-xs bg-gray-500 text-gray-900 rounded-sm px-2">
          {props.provider}
        </span>
      </div>
    </div>
  );
};

const SettingsOAuth2List: Component = () => {
  const [workspace] = useWorkspace();
  const module: Module = "OAuth2";

  const getItems = createMemo(() => {
    if (!workspace.isFetching[module] && !!workspace.isReady[module]) {
      return workspace[module];
    }
    return [];
  });

  return (
    <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
      <For each={getItems()}>{(item) => <SettingsOAuth2Item {...item} />}</For>
    </div>
  );
};

export default SettingsOAuth2List;
