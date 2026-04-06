/**
 * @file OperationModals.tsx
 *
 * Used to keep track of the current operation that MBF is carrying out.
 * "Operations" are mutually exclusive, so e.g. if the app permissions are being changed,
 * we cannot have a mod installation in progress, for example.
 */

import { ScaleLoader } from "react-spinners";
import { LogWindow, LogWindowControls } from "./LogWindow";
import { ErrorModal } from "./Modal";
import React, { useLayoutEffect } from "react";
import { Log } from "../Logging";

/**
 * An error that occured within a particular operation.
 */
export interface OperationError {
  title: string;
  error: string;
}

const state: OperationModalsData = ({
  currentOperation: null,
  currentError: null,
  statusText: null,
  logsManuallyOpen: false,
  setCurrentOperation: undefined,
  setCurrentError: undefined,
  setStatusText: undefined,
  setLogsManuallyOpen: undefined,
  mounted: false,
});

interface OperationModalsData {
  /** Name of the current ongoing operation */
  currentOperation: string | null;
  currentError: OperationError | null;

  /** Progress/status text for the ongoing operation */
  statusText: string | null;

  /** Whether or not the logs have been manually opened by the user. */
  logsManuallyOpen: boolean;
  
  mounted: boolean;
  
  setCurrentOperation?: React.Dispatch<React.SetStateAction<string | null>>;
  setCurrentError?: React.Dispatch<React.SetStateAction<OperationError | null>>;
  setStatusText?: React.Dispatch<React.SetStateAction<string | null>>;
  setLogsManuallyOpen?: React.Dispatch<React.SetStateAction<boolean>>;
}

// Component that displays the log window when an operation is in progress, and displays errors when the operation failed.
export function OperationModals() {
  const [currentOperation, setCurrentOperation] = React.useState<string | null>(null);
  const [currentError, setCurrentError] = React.useState<OperationError | null>(null);
  const [statusText, setStatusText] = React.useState<string | null>(null);
  const [logsManuallyOpen, setLogsManuallyOpen] = React.useState<boolean>(false);
  
  state.currentOperation = currentOperation;
  state.currentError = currentError;
  state.statusText = statusText;
  state.logsManuallyOpen = logsManuallyOpen;
  
  useLayoutEffect(() => {
    if (state.mounted) {
      throw new Error("Multiple OperationModals components mounted. There should only be one.");
    }
    
    state.setCurrentOperation = setCurrentOperation;
    state.setCurrentError = setCurrentError;
    state.setStatusText = setStatusText;
    state.setLogsManuallyOpen = setLogsManuallyOpen;
    state.mounted = true;

    return () => {
      state.setCurrentOperation = undefined;
      state.setCurrentError = undefined;
      state.setStatusText = undefined;
      state.setLogsManuallyOpen = undefined;
      state.mounted = false;
    };
  });

  const canClose = logsManuallyOpen && currentError === null;
  const needSyncModal =
    (logsManuallyOpen || currentOperation !== null) && currentError === null;

  return (
    <>
      <SyncingModal
        isVisible={needSyncModal}
        title={currentOperation ?? "Log output"}
        subtext={statusText}
        onClose={
          canClose ? () => (setLogsManuallyOpen(false)) : undefined
        }
      />
      <ErrorModal
        isVisible={currentError !== null}
        title={currentError?.title ?? ""}
        description={currentError?.error}
        onClose={() => (setCurrentError(null))}
      ></ErrorModal>
    </>
  );
}

export namespace OperationModals {
  export let currentOperation: string | null;
  export let currentError: OperationError | null;
  export let statusText: string | null;
  export let logsManuallyOpen: boolean;

  /**
   * Creates a function that can be used to set whether or not a particular operation is currently in progress.
   */
  export function useSetWorking(operationName: string): (working: boolean) => void {
    return function setWorking(working) {
      if (working) {
        OperationModals.currentOperation = operationName;
      } else {
        OperationModals.statusText = null;
        OperationModals.currentOperation = null;
      }
    };
  }

  /**
   * Creates a function that can be used to set an error when a particular operation failed.
   */
  export function useSetError(errorTitle: string): (error: unknown | null) => void {
    return function setError(error) {
      if (error === null) {
        OperationModals.currentError = null;
      } else {
        Log.error(`${errorTitle}: ${String(error)}`);
        OperationModals.currentError = {
          title: errorTitle,
          error: String(error),
        };
      }
    };
  }

  export async function wrapOperation(
    operationName: string,
    errorModalTitle: string,
    operation: () => Promise<void>
  ) {
    const setWorking = useSetWorking(operationName);
    const setError = useSetError(errorModalTitle);

    setWorking(true);
    try {
      await operation();
    } catch (error) {
      Log.error(errorModalTitle + ": " + error);
      setError(error);
    } finally {
      setWorking(false);
    }
  }
}

Object.defineProperties(OperationModals, {
  currentOperation: {
    get: () => state.currentOperation,
    set: (value) => state.setCurrentOperation!(value),
    enumerable: true,
    configurable: false,
  },
  currentError: {
    get: () => state.currentError,
    set: (value) => state.setCurrentError!(value),
    enumerable: true,
    configurable: false,
  },
  statusText: {
    get: () => state.statusText,
    set: (value) => state.setStatusText!(value),
    enumerable: true,
    configurable: false, 
  },
  logsManuallyOpen: {
    get: () => state.logsManuallyOpen,
    set: (value) => state.setLogsManuallyOpen!(value),
    enumerable: true,
    configurable: false,
  }
})

function SyncingModal({
  isVisible,
  title,
  subtext,
  onClose,
}: {
  isVisible: boolean;
  title: string;
  subtext: string | null;
  onClose?: () => void;
}) {
  if (isVisible) {
    return (
      <div className="modalBackground coverScreen">
        <div className="modal container screenWidth">
          <div className="syncingWindow">
            <div className="syncingTitle">
              <h2>{title}</h2>
              {onClose === undefined && (
                <ScaleLoader color={"white"} height={20} />
              )}
              <LogWindowControls onClose={onClose} />
            </div>
            {subtext && <span className="syncingSubtext">{subtext}</span>}

            <LogWindow />
          </div>
        </div>
      </div>
    );
  } else {
    return <div className="modalBackground modalClosed coverScreen"></div>;
  }
}
