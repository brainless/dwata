import React, { useEffect, useContext } from "react";

import { QueryContext } from "utils";
import { useData, useQuerySpecification } from "services/store";
import QueryLoader from "./QueryLoader";

const extractQuerySpecification = (item) => {
  const innerQS = JSON.parse(item[2]);

  return {
    sourceLabel: innerQS.source_label,
    tableName: innerQS.table_name,
    columnsSelected: innerQS.columns,
    orderBy: innerQS.order_by,
    filterBy: innerQS.filter_by,
    offset: innerQS.offset,
    limit: innerQS.limit,
    fetchNeeded: true,
  };
};

const InnerLoader = ({ children }) => {
  const queryContext = useContext(QueryContext);
  const data = useData((state) => state[queryContext.key]);
  const querySpecification = useQuerySpecification(
    (state) => state[queryContext.key]
  );
  const initiateQuerySpecification = useQuerySpecification(
    (state) => state.initiateQuerySpecification
  );
  const context = {
    key: `saved-query-${querySpecification.pk}`,
  };
  initiateQuerySpecification(
    context.key,
    extractQuerySpecification(data.rows[0])
  );

  return (
    <QueryContext.Provider value={context}>
      <QueryLoader>{children}</QueryLoader>
    </QueryContext.Provider>
  );
};

export default ({ children }) => {
  // We made this small separate component just for the separate useEffect used here
  const queryContext = useContext(QueryContext);
  const fetchData = useData((state) => state.fetchData);
  const querySpecification = useQuerySpecification(
    (state) => state[queryContext.key]
  );
  const data = useData((state) => state[queryContext.key]);

  useEffect(() => {
    if (!!querySpecification && !!querySpecification.fetchNeeded) {
      fetchData(queryContext.key, querySpecification);
    }
  }, [queryContext.key, querySpecification.fetchNeeded]);

  if (
    !(data && data.isReady && querySpecification && querySpecification.isReady)
  ) {
    return <div>Loading data for Saved Query...</div>;
  }

  return <InnerLoader>{children}</InnerLoader>;
};
