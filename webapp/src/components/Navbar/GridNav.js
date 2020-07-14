import React, { useEffect, useContext, Fragment } from "react";

import { QueryContext } from "utils";
import { useGlobal, useApps, useData } from "services/store";

export default ({
  cacheKey,
  hasColumnsSpecified,
  hasFiltersSpecified,
  hasOrderingSpecified,
  showNotes,
  toggleActions,
  togglePinnedRecords,
}) => {
  const queryContext = useContext(QueryContext);
  const showPinnedRecords = useGlobal((state) => state.showPinnedRecords);
  const toggleFilterEditor = useGlobal((state) => state.toggleFilterEditor);
  const toggleColumnSelector = useGlobal((state) => state.toggleColumnSelector);
  const toggleOrderEditor = useGlobal((state) => state.toggleOrderEditor);
  const data = useData((state) => state[queryContext.key]);
  const fetchApps = useApps((state) => state.fetchApps);
  const isNoteAppEnabled = useApps((state) => state.isNoteAppEnabled);
  const isRecordPinAppEnabled = useApps((state) => state.isRecordPinAppEnabled);
  useEffect(() => {
    fetchApps();
  }, [fetchApps]);

  if (!(data && data.isReady)) {
    return null;
  }
  const { selectedRowList } = data;

  const handleNotesClick = (event) => {
    event.preventDefault();
    showNotes(cacheKey);
  };
  const handleActionsClick = (event) => {
    event.preventDefault();
    toggleActions();
  };
  const handlePinClick = (event) => {
    event.preventDefault();
    togglePinnedRecords();
  };

  return (
    <Fragment>
      <div className="navbar-item">
        <div className="field">
          <p className="control">
            <button
              className={`button ${
                selectedRowList.length > 0 ? " is-success" : ""
              }`}
              disabled={selectedRowList.length === 0}
              onClick={handleActionsClick}
            >
              <span className="icon">
                <i className="far fa-check-square" />
              </span>
              &nbsp; Actions
            </button>
          </p>
        </div>
      </div>

      <div className="navbar-item">
        <div className="buttons has-addons">
          {isNoteAppEnabled ? (
            <button className="button" onClick={handleNotesClick}>
              <i className="far fa-sticky-note" />
              &nbsp; Notes
            </button>
          ) : null}
          {isRecordPinAppEnabled ? (
            <button
              className={`button${showPinnedRecords ? " is-success" : ""}`}
              onClick={handlePinClick}
            >
              <i className="fas fa-thumbtack" />
              &nbsp; Pins
            </button>
          ) : null}
        </div>
      </div>

      <div className="navbar-item">
        <div className="buttons has-addons">
          <button
            className={`button${hasColumnsSpecified ? " is-spec" : ""}`}
            onClick={toggleColumnSelector}
          >
            <i className="fas fa-columns" />
            &nbsp;Columns
          </button>
          <button
            className={`button${hasFiltersSpecified ? " is-spec" : ""}`}
            onClick={toggleFilterEditor}
          >
            <i className="fas fa-filter" />
            &nbsp;Filters
          </button>
          <button
            className={`button${hasOrderingSpecified ? " is-spec" : ""}`}
            onClick={toggleOrderEditor}
          >
            <i className="fas fa-sort" />
            &nbsp;Ordering
          </button>
        </div>
      </div>
    </Fragment>
  );
};

/*
const mapStateToProps = (state, props) => {
  const match = getSourceFromPath(props.location.pathname);
  const { sourceId, tableName } =
    match != null ? match.params : { sourceId: null, tableName: null };
  let isInTable = false;
  let hasColumnsSpecified = false;
  let hasFiltersSpecified = false;
  let hasOrderingSpecified = false;
  const cacheKey = getCacheKey(state);

  if (sourceId && tableName) {
    isInTable = true;
    if (
      state.schema.isReady &&
      state.schema.sourceId === parseInt(sourceId) &&
      state.browser.isReady &&
      state.browser.cacheKey === cacheKey &&
      state.querySpecification.isReady &&
      state.querySpecification.cacheKey === cacheKey
    ) {
      hasColumnsSpecified =
        state.querySpecification.columnsSelected.length !==
        state.schema.rows.find((x) => x.table_name === tableName).columns
          .length;
      hasFiltersSpecified =
        Object.keys(state.querySpecification.filterBy).length > 0;
      hasOrderingSpecified =
        Object.keys(state.querySpecification.orderBy).length > 0;
    }
  }
  const { isNoteAppEnabled, isRecordPinAppEnabled } = state.apps;

  return {
    isSourceFetching: state.source.isFetching,
    sourceId,
    tableName,
    isFilterEnabled: sourceId && tableName,
    isInTable,
    hasColumnsSpecified,
    hasFiltersSpecified,
    hasOrderingSpecified,
    selectedRowList: state.browser.selectedRowList,
    cacheKey,
    isNoteAppEnabled,
    isRecordPinAppEnabled,
    showPinnedRecords: state.global.showPinnedRecords,
  };
};

export default withRouter(
  connect(mapStateToProps, {
    toggleSidebar,
    toggleFilterEditor,
    toggleColumnSelector,
    toggleOrderEditor,
    showNotes,
    getApps,
    toggleActions,
    togglePinnedRecords,
  })(Navbar)
);
*/
