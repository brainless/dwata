import { Component } from "solid-js";
import { useParams } from "@solidjs/router";
import Form from "../interactable/ConfiguredForm";
import { Module } from "../../api_types/Module";

const Oauth2AppForm: Component = () => {
  const params = useParams();

  return (
    <div class="max-w-screen-sm">
      <Form
        module={"OAuth2App" as Module}
        existingItemId={!!params.id ? parseInt(params.id) : undefined}
        postSaveNavigateTo={"/settings"}
      />
    </div>
  );
};

export default Oauth2AppForm;
