import { useContext } from "react";

import { QueryContext } from "utils";
import EmbeddedTableHead from "./EmbeddedTableHead";
import EmbeddedTableBody from "./EmbeddedTableBody";

/**
 * This component creates a new context inside which we display an embedded table.
 * Embedded grids have less functionality but are otherwise visually similar to a regular Grid.
 *
 */
function EmbeddedTable({ embedContext }) {
  const queryContext = useContext(QueryContext);
  const ctx = {
    isEmbedded: true,
    ...embedContext,
    key: queryContext.key,
  };

  return (
    <QueryContext.Provider value={ctx}>
      <>
        <div>
          <table className="font-content tracking-normal bg-white border-collapse">
            <thead>
              <EmbeddedTableHead />
            </thead>

            <tbody>
              <EmbeddedTableBody />
            </tbody>
          </table>
        </div>
      </>
    </QueryContext.Provider>
  );
}

export default EmbeddedTable;
