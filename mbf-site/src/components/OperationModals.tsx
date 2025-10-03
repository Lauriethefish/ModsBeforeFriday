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
import React from "react";
import { proxy, useSnapshot } from "valtio";
import { Log } from "../Logging";
import { definePropertiesFromSource } from "../definePropertiesFromSource";

/**
 * An error that occured within a particular operation.
 */
export interface OperationError {
  title: string;
  error: string;
}

const modalState = proxy<OperationModalsData>({
  currentOperation: null,
  currentError: null,
  statusText: null,
  logsManuallyOpen: false,
});

interface OperationModalsData {
  /** Name of the current ongoing operation */
  currentOperation: string | null;
  currentError: OperationError | null;

  /** Progress/status text for the ongoing operation */
  statusText: string | null;

  /** Whether or not the logs have been manually opened by the user. */
  logsManuallyOpen: boolean;
}

interface OperationModalsActions {
  /** Creates a function that can be used to set whether or not a particular operation is currently in progress. */
  useSetWorking: typeof useSetWorking;

  /** Creates a function that can be used to set an error when a particular operation failed. */
  useSetError: typeof useSetError;

  /** Used to wrap a particular operation while displaying the logging window and any errors if appropriate. */
  wrapOperation: typeof wrapOperation;
}

/**
 * Creates a function that can be used to set whether or not a particular operation is currently in progress.
 */
function useSetWorking(
  operationName: string
): (working: boolean) => void {
  return function setWorking(working) {
    if (working) {
      modalState.currentOperation = operationName;
    } else {
      modalState.statusText = null;
      modalState.currentOperation = null;
    }
  };
}

/**
 * Creates a function that can be used to set an error when a particular operation failed.
 */
function useSetError(
  errorTitle: string
): (error: unknown | null) => void {
  return function setError(error) {
    if (error === null) {
      modalState.currentError = null;
    } else {
      Log.error(`${errorTitle}: ${String(error)}`);
      modalState.currentError = {
        title: errorTitle,
        error: String(error),
      };
    }
  };
}

async function wrapOperation(
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

// Component that displays the log window when an operation is in progress, and displays errors when the operation failed.
export function OperationModals() {
  const { currentOperation, currentError, statusText, logsManuallyOpen } = useSnapshot(modalState);

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
          canClose
            ? () => (modalState.logsManuallyOpen = false)
            : undefined
        }
      />
      <ErrorModal
        isVisible={currentError !== null}
        title={currentError?.title ?? ""}
        description={currentError?.error}
        onClose={() => (modalState.currentError = null)}
      ></ErrorModal>
    </>
  );
}

export namespace OperationModals  {
  export let currentOperation: string | null;
  export let currentError: OperationError | null;
  export let statusText: string | null;
  export let logsManuallyOpen: boolean;
  export let useSetWorking: OperationModalsActions["useSetWorking"];
  export let useSetError: OperationModalsActions["useSetError"];
  export let wrapOperation: OperationModalsActions["wrapOperation"];
}

definePropertiesFromSource(OperationModals, modalState);
definePropertiesFromSource(OperationModals, {useSetWorking, useSetError, wrapOperation}, undefined, true);

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
