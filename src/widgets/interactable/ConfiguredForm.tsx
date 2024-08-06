import {
  Component,
  For,
  JSX,
  createComputed,
  createResource,
  createSignal,
  onMount,
} from "solid-js";
import Heading from "../typography/Heading";
import Button from "./Button";
import FormField from "./FormField";
import { useUserInterface } from "../../stores/userInterface";
import { Configuration } from "../../api_types/Configuration";
import { IFormFieldValue } from "../../utils/types";
import { useNavigate } from "@solidjs/router";
import { Module } from "../../api_types/Module";
import { invoke } from "@tauri-apps/api/core";
import { ModuleDataRead } from "../../api_types/ModuleDataRead";
import { InsertUpdateResponse } from "../../api_types/InsertUpdateResponse";
import { nextTaskStore } from "../../stores/nextTask";
import { NextStep } from "../../api_types/NextStep";

interface IPropTypes {
  module: Module;
  existingItemId?: number;
  initialData?: {
    [key: string]: IFormFieldValue;
  };
  postSaveNavigateTo?: string;
  initiateNextTask?: boolean;
  title?: string;
  showPrelude?: boolean;
}

const Form: Component<IPropTypes> = (props) => {
  const [formData, setFormData] = createSignal<{
    [key: string]: IFormFieldValue;
  }>({});
  // const [dirty, setDirty] = createSignal<Array<string>>([]);
  const navigate = useNavigate();
  const [_, { getColors }] = useUserInterface();

  const [
    config,
    { mutate: mutateModuleConfiguration, refetch: refetchModuleConfiguration },
  ] = createResource<Configuration | undefined>(
    async (_source, { value, refetching }) => {
      if (typeof refetching === "object") {
        // We are refetching the configuration on change
        let response = await invoke<NextStep>(
          "module_insert_or_update_on_change",
          {
            module: props.module,
            data: refetching,
          },
        );

        if (!!response && response.type === "Configure") {
          return response.data as Configuration;
        } else if (!!response && response.type === "Continue") {
          return value;
        }
      } else {
        let response = await invoke<NextStep>(
          "module_insert_or_update_initiate",
          {
            module: props.module,
          },
        );
        if (!!response && response.type === "Configure") {
          return response.data as Configuration;
        }
      }
    },
  );
  const [moduleData, { mutate: _mm, refetch: refetchModuleData }] =
    createResource<ModuleDataRead>(async () => {
      return await invoke<ModuleDataRead>("read_module_item_by_pk", {
        module: props.module,
        pk: props.existingItemId,
      });
    });

  onMount(() => {
    refetchModuleConfiguration();

    if (props.existingItemId) {
      refetchModuleData();
    }
  });

  createComputed((isSet) => {
    // This function loads data when we are editing existing items
    if (!props.existingItemId || isSet) {
      return false;
    }

    if (
      !isSet &&
      moduleData.state === "ready" &&
      moduleData() !== undefined &&
      props.module in moduleData()!
    ) {
      console.log(moduleData());

      let _data: { [key: string]: IFormFieldValue } = {};
      let fieldNames = config()!.fields.map((f) => f.name);
      for (const [key, value] of Object.entries(
        moduleData()![props.module as keyof ModuleDataRead],
      )) {
        if (fieldNames.includes(key)) {
          _data[key] = value as IFormFieldValue;
        }
      }

      setFormData((state) => ({
        ...state,
        ..._data,
      }));

      // Once we set existing data, we return true so this computation will not run again
      return true;
    }
    return false;
  });

  createComputed(() => {
    // This function set inidial values from form configuration
    if (config.state === "ready" && !!config()) {
      for (const field of config()!.fields) {
        if (!!field.defaultValue) {
          let value = undefined;
          if ("Text" in field.defaultValue) {
            value = field.defaultValue.Text;
          } else if ("ID" in field.defaultValue) {
            value = field.defaultValue.ID;
          }

          if (!!value) {
            setFormData((state) => ({
              ...state,
              [field.name]: value,
            }));

            // setDirty((state) =>
            //   state.includes(field.name) ? state : [...state, field.name],
            // );
          }
        }
      }
    }
  });

  const handleChange = (name: string, value: IFormFieldValue) => {
    setFormData((state) => ({
      ...state,
      [name]: value,
    }));

    // setDirty((state) => (state.includes(name) ? state : [...state, name]));

    // Let's check if the form configuration has changes for this change in form data
    refetchModuleConfiguration({
      [props.module]: formData(),
    });
  };

  const submitForm = async () => {
    const data = {
      [props.module]: formData(),
    };

    if (!!props.existingItemId) {
      console.info(
        `Submitting form data to update module ${props.module}, item ID ${props.existingItemId}, data:`,
        formData(),
      );
      const response = await invoke<InsertUpdateResponse>(
        "update_module_item",
        {
          pk: props.existingItemId,
          data,
        },
      );
      if (!!response && !!props.postSaveNavigateTo) {
        navigate(props.postSaveNavigateTo);
      }
    } else {
      console.info(
        `Submitting form data to insert into module: ${props.module}, data:`,
        formData(),
      );

      const response = await invoke<InsertUpdateResponse>(
        "insert_module_item",
        {
          data,
        },
      );
      if (!!response) {
        if (!!props.initialData && !!response.nextTask) {
          // console.log(response);

          nextTaskStore[1].initiateTask({
            name: response.nextTask,
            arguments: response.arguments || undefined,
          });
          if (!!props.postSaveNavigateTo) {
            navigate(
              props.postSaveNavigateTo + "?nextTask=" + response.nextTask,
            );
          }
        }
        if (!!props.postSaveNavigateTo) {
          navigate(props.postSaveNavigateTo);
        }
      }
    }
  };

  const handleSubmit: JSX.EventHandler<HTMLFormElement, Event> = async (
    event,
  ) => {
    event.preventDefault();
    const data = {
      [props.module]: formData(),
    };
    let response = await invoke<NextStep>("module_insert_or_update_next_step", {
      module: props.module,
      data,
    });

    if (!!response && response.type === "Configure") {
      // We have been asked to update the form with new configuration
      mutateModuleConfiguration(response.data as Configuration);
    } else {
      await submitForm();
    }
  };

  createComputed(async () => {
    if (config.state === "ready" && !!config() && config()?.submitImplicitly) {
      await submitForm();
    }
  });

  const Inner = () => {
    const Buttons: Component = () => {
      return (
        <For each={config()?.buttons}>{(button) => <Button {...button} />}</For>
      );
    };

    return (
      <form onSubmit={handleSubmit}>
        <For each={config()?.fields}>
          {(field) => (
            <>
              <FormField
                {...field}
                onChange={handleChange}
                value={
                  field.name in formData() ? formData()[field.name] : undefined
                }
              />
              <div class="mt-4" />
            </>
          )}
        </For>

        <Buttons />
      </form>
    );
  };

  if (props.showPrelude !== undefined && !props.showPrelude) {
    return <Inner />;
  } else {
    return (
      <div
        class="w-full rounded-md border"
        style={{
          "background-color": getColors().colors["inlineChat.background"],
          "border-color": getColors().colors["inlineChat.border"],
        }}
      >
        <div
          class="px-2 py-2 rounded-md rounded-b-none border-b"
          style={{
            "border-color": getColors().colors["editorWidget.border"],
          }}
        >
          <Heading size="xl">{config()?.title || props.title}</Heading>

          <p style={{ color: getColors().colors["editor.foreground"] }}>
            {config()?.description}
          </p>
        </div>

        <div class="px-2 pt-2 pb-3 rounded-md rounded-t-none">
          <Inner />
        </div>
      </div>
    );
  }
};

export default Form;
