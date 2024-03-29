import { Component, For, createMemo, onMount } from "solid-js";
import SidebarHeading from "../navigation/SidebarHeading";
import { useWorkspace } from "../../stores/workspace";
import { useSearchParams } from "@solidjs/router";

const SourceList: Component = () => {
  const [workspace, { readConfigFromAPI }] = useWorkspace();
  const [searchParams] = useSearchParams();

  onMount(async () => {
    await readConfigFromAPI();
  });

  const dataSources = createMemo(() => {
    if (!workspace.isFetching && !!workspace.isReady) {
      return workspace.dataSourceList;
    }
    return [];
  });

  return (
    <>
      <For each={dataSources()}>
        {(dataSource) => {
          const label = dataSource.label || dataSource.sourceName;

          return (
            <SidebarHeading
              label={label}
              icon="fa-solid fa-database"
              href={`/browse?dataSourceId=${dataSource.id}`}
              isActive={
                !!searchParams.dataSouceId &&
                dataSource.id == searchParams.dataSouceId
              }
              infoTag={dataSource.sourceType}
            />
          );
        }}
      </For>
    </>
  );
};

export default SourceList;
