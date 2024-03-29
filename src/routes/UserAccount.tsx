import { Component, createComputed, createSignal, onMount } from "solid-js";
import { IFormField } from "../utils/types";
import Form from "../widgets/interactable/Form";
import { invoke } from "@tauri-apps/api/core";
import { useUser } from "../stores/user";
import Heading from "../widgets/typography/Heading";
import { useNavigate } from "@solidjs/router";

interface IUserAccountFormData {
  firstName: string;
  lastName?: string;
  email?: string;
}

const UserAccount: Component = () => {
  const [user, { fetchCurrentUser }] = useUser();
  const [formData, setFormData] = createSignal<IUserAccountFormData>({
    firstName: "",
  });
  const navigate = useNavigate();

  onMount(async () => {
    await fetchCurrentUser();
  });

  createComputed(() => {
    setFormData({
      firstName: user.account?.firstName || "",
      lastName: user.account?.lastName || "",
      email: user.account?.email || "",
    });
  });

  const formFields: Array<IFormField> = [
    {
      name: "firstName",
      label: "First Name (required)",
      fieldType: "singleLineText",
      isRequired: true,
    },
    {
      name: "lastName",
      label: "Last Name",
      fieldType: "singleLineText",
    },
    {
      name: "email",
      label: "Email",
      fieldType: "singleLineText",
    },
  ];

  const handleSubmit = async () => {
    await invoke("save_user", {
      firstName: formData().firstName,
      lastName: formData().lastName,
      email: formData().email,
    });
    navigate("/");
  };

  return (
    <div class="max-w-screen-sm">
      <Heading size="3xl">Account</Heading>
      <div class="mb-4" />

      <Form
        title="My account"
        formFields={formFields}
        submitButtomLabel="Save"
        handleSubmit={handleSubmit}
        formData={formData()}
        setFieldInput={setFormData}
      ></Form>
    </div>
  );
};

export default UserAccount;
