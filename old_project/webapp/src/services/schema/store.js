import create from "zustand";

import { transformData } from "utils";
import { schemaURL } from "services/urls";
import apiClient from "utils/apiClient";

const initialState = {
  columns: [],
  rows: [],

  isFetching: false,
  isReady: false,
};

const initiateFetch = () => ({
  ...initialState,
  isFetching: true,
});

const completeFetch = (payload) => ({
  columns: payload.columns,
  rows: payload.rows.map((row) => transformData(payload.columns, row)),
  isFetching: false,
  isReady: true,
  fetchedAt: Math.round(new Date().getTime() / 1000),
});

export default create((set, get) => ({
  fetchSchema: async (sourceLabel) => {
    if (!sourceLabel) {
      return;
    }
    if (get()[sourceLabel] && get()[sourceLabel].isFetching) {
      return;
    }
    if (
      get()[sourceLabel] &&
      Math.round(new Date().getTime() / 1000) - get()[sourceLabel].fetchedAt <
        600
    ) {
      return;
    }

    !get()[sourceLabel] &&
      set(() => ({
        [sourceLabel]: initiateFetch(),
      }));

    const response = await apiClient.get(`${schemaURL}/${sourceLabel}`);
    set(() => ({
      [sourceLabel]: completeFetch(response.data),
    }));
  },
}));
