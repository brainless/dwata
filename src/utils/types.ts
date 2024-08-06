import { JSX } from "solid-js";
import { FormField } from "../api_types/FormField";
import { ContentType } from "../api_types/ContentType";
import { ContentSpec } from "../api_types/ContentSpec";
import { Content } from "../api_types/Content";
import { DirectorySource } from "../api_types/DirectorySource";
import { DatabaseSource } from "../api_types/DatabaseSource";
import { AIIntegration } from "../api_types/AIIntegration";
import { EmailAccount } from "../api_types/EmailAccount";
import { Module } from "../api_types/Module";
import { OAuth2App } from "../api_types/OAuth2App";

interface ILabel {
  id: number;
  label: string;
  path: string;
}

type TRowValue = any;

interface IProviderPropTypes {
  children: JSX.Element;
}

interface APIGridData {
  source: string;
  schema: string | null;
  table: string | null;
  rows: Array<Array<any>>;
}

type IFormFieldValue = string | number | Array<string> | undefined;
type IFormData = { [key: string]: IFormFieldValue };

export interface IChoicesWithHeading {
  name: string;
  choices: Array<[string, string]>;
}

interface IFormField extends FormField {
  size?: "xs" | "sm" | "base" | "lg";
  displayBlock?: boolean;
  value?: IFormFieldValue;
  onChange?: (name: string, value: IFormFieldValue) => void;
  onFocus?: () => void;
}

interface IWorkspace {
  DirectorySource: Array<DirectorySource>;
  DatabaseSource: Array<DatabaseSource>;
  AIIntegration: Array<AIIntegration>;
  OAuth2App: Array<OAuth2App>;
  EmailAccount: Array<EmailAccount>;

  isReady: Partial<{ [key in Module]: boolean }>;
  isFetching: Partial<{ [key in Module]: boolean }>;
}

interface INextTask {
  name: string;
  arguments?: Array<[string, ContentType, Content]>;
}

type IHeterogenousContent = [ContentType, ContentSpec, Content];

type uiThemes = "gitHubDark" | "gitHubLight";

export type {
  ILabel as IChatRoom,
  TRowValue,
  IProviderPropTypes,
  APIGridData,
  IFormFieldValue,
  IFormData,
  IFormField,
  IWorkspace,
  INextTask,
  IHeterogenousContent,
  uiThemes,
};
