import { Component, For, createMemo } from "solid-js";
import { useDirectory } from "../../stores/directory";
import FileItem from "./FileItem";

const FileList: Component = () => {
  const [store] = useDirectory();

  const getFileList = createMemo(() => store.fileList);

  return (
    <div class="overflow-y-auto">
      <For each={getFileList()}>{(file) => <FileItem {...file} />}</For>
    </div>
  );
};

export default FileList;
