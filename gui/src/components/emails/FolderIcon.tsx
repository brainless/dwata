import {
  HiOutlineInbox,
  HiOutlinePaperAirplane,
  HiOutlineDocument,
  HiOutlineStar,
  HiOutlineArchiveBox,
  HiOutlineTrash,
  HiOutlineFolder,
} from "solid-icons/hi";
import type { Component } from "solid-js";

interface FolderIconProps {
  folderType: string | null;
  class?: string;
}

const FolderIcon: Component<FolderIconProps> = (props) => {
  const iconClass = () => props.class || "w-5 h-5";

  switch (props.folderType?.toLowerCase()) {
    case 'inbox':
      return <HiOutlineInbox class={iconClass()} />;
    case 'sent':
      return <HiOutlinePaperAirplane class={iconClass()} />;
    case 'drafts':
      return <HiOutlineDocument class={iconClass()} />;
    case 'starred':
    case 'flagged':
      return <HiOutlineStar class={iconClass()} />;
    case 'archive':
      return <HiOutlineArchiveBox class={iconClass()} />;
    case 'trash':
    case 'deleted':
      return <HiOutlineTrash class={iconClass()} />;
    default:
      return <HiOutlineFolder class={iconClass()} />;
  }
};

export default FolderIcon;
