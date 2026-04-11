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
import React, {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useLayoutEffect,
  useState,
} from "react";
import { Log } from "../Logging";

/**
 * An error that occured within a particular operation.
 */
export interface OperationError {
  title: string;
  error: string;
}

interface OperationModalsData {
  /** Name of the current ongoing operation */
  currentOperation: string | null;
  currentError: OperationError | null;

  /** Progress/status text for the ongoing operation */
  statusText: string | null;

  /** Whether or not the logs have been manually opened by the user. */
  logsManuallyOpen: boolean;
  setCurrentOperation: React.Dispatch<React.SetStateAction<string | null>>;
  setCurrentError: React.Dispatch<React.SetStateAction<OperationError | null>>;
  setStatusText: React.Dispatch<React.SetStateAction<string | null>>;
  setLogsManuallyOpen: React.Dispatch<React.SetStateAction<boolean>>;

  /** Creates a function that can be used to set whether or not a particular operation is currently in progress. */
  useSetWorking: (operationName: string) => (working: boolean) => void;

  /** Creates a function that can be used to set an error when a particular operation failed. */
  useSetError: (errorTitle: string) => (error: unknown | null) => void;

  /** Used to wrap a particular operation while displaying the logging window and any errors if appropriate. */
  wrapOperation: (
    operationName: string,
    errorModalTitle: string,
    operation: () => Promise<void>
  ) => Promise<void>;
}

interface OperationModalsComponents {
  OperationModalContextProvider: React.FC<React.PropsWithChildren>;
  OperationModals: React.FC;
}

const OperationModalsContext =
  createContext<Readonly<OperationModalsData> | null>(null);

export function useOperationModals(): OperationModalsComponents {
  const _OperationModalContextProvider = useCallback<
    React.FC<React.PropsWithChildren>
  >(function OperationModalContextProvider({ children }) {
    const context = useContext(OperationModalsContext);
    if (context !== null) {
      throw new Error("OperationModalsContextProvider cannot be nested.");
    }

    return (
      <OperationModalsContext.Provider
        value={{
          currentOperation: null,
          currentError: null,
          statusText: null,
          logsManuallyOpen: false,
          setCurrentOperation: () => undefined,
          setCurrentError: () => undefined,
          setStatusText: () => undefined,
          setLogsManuallyOpen: () => undefined,
          useSetWorking: () => () => {},
          useSetError: () => () => {},
          wrapOperation: async () => Promise.resolve(),
        }}
      >
        {children}
      </OperationModalsContext.Provider>
    );
  }, []);

  return {
    OperationModalContextProvider: _OperationModalContextProvider,
    OperationModals,
  };
}

export const useOperationModalsContext = () => {
  const context = useContext(OperationModalsContext);

  if (context === null) {
    throw new Error(
      "useOperationModalsContext must be used within an OperationModalContextProvider."
    );
  }

  return context;
};

// Component that displays the log window when an operation is in progress, and displays errors when the operation failed.
function OperationModals() {
  const [currentOperation, setCurrentOperation] = useState<string | null>(null);
  const [currentError, setCurrentError] = useState<OperationError | null>(null);
  const [statusText, setStatusText] = useState<string | null>(null);
  const [logsManuallyOpen, setLogsManuallyOpen] = useState(false);
  const context = useContext(OperationModalsContext)! as OperationModalsData & {
    modalRenderer: boolean;
  };

  if (context === null) {
    throw new Error(
      "OperationModals must be used within an OperationModalContextProvider."
    );
  }

  /**
   * Creates a function that can be used to set whether or not a particular operation is currently in progress.
   */
  const _useSetWorking = useCallback(
    function useSetWorking(operationName: string): (working: boolean) => void {
      return function setWorking(working) {
        if (working) {
          setCurrentOperation(operationName);
        } else {
          setStatusText(null);
          setCurrentOperation(null);
        }
      };
    },
    [setCurrentOperation, setStatusText]
  );

  /**
   * Creates a function that can be used to set an error when a particular operation failed.
   */
  const _useSetError = useCallback(
    function useSetError(errorTitle: string): (error: unknown | null) => void {
      return function setError(error) {
        if (error === null) {
          setCurrentError(null);
        } else {
          Log.error(`${errorTitle}: ${String(error)}`, error);
          setCurrentError({
            title: errorTitle,
            error: String(error),
          });
        }
      };
    },
    [setCurrentError]
  );

  const _wrapOperation = useCallback(
    async function wrapOperation(
      operationName: string,
      errorModalTitle: string,
      operation: () => Promise<void>
    ) {
      const setWorking = _useSetWorking(operationName);
      const setError = _useSetError(errorModalTitle);

      setWorking(true);
      try {
        await operation();
      } catch (error) {
        Log.error(errorModalTitle + ": " + error, error);
        setError(error);
      } finally {
        setWorking(false);
      }
    },
    [_useSetWorking, _useSetError]
  );

  useLayoutEffect(() => {
    context.currentOperation = currentOperation;
  }, [context, currentOperation]);

  useLayoutEffect(() => {
    context.currentError = currentError;
  }, [context, currentError]);

  useLayoutEffect(() => {
    context.statusText = statusText;
  }, [context, statusText]);

  useLayoutEffect(() => {
    context.logsManuallyOpen = logsManuallyOpen;
  }, [context, logsManuallyOpen]);

  useLayoutEffect(() => {
    context.setCurrentOperation = setCurrentOperation;
  }, [context, setCurrentOperation]);

  useLayoutEffect(() => {
    context.setCurrentError = setCurrentError;
  }, [context, setCurrentError]);

  useLayoutEffect(() => {
    context.setStatusText = setStatusText;
  }, [context, setStatusText]);

  useLayoutEffect(() => {
    context.setLogsManuallyOpen = setLogsManuallyOpen;
  }, [context, setLogsManuallyOpen]);

  useLayoutEffect(() => {
    context.useSetWorking = _useSetWorking;
  }, [context, _useSetWorking]);

  useLayoutEffect(() => {
    context.useSetError = _useSetError;
  }, [context, _useSetError]);

  useLayoutEffect(() => {
    context.wrapOperation = _wrapOperation;
  }, [context, _wrapOperation]);

  useEffect(() => {
    context.modalRenderer = true;

    return () => {
      context.modalRenderer = false;
    };
  }, [context]);

  const canClose = logsManuallyOpen && currentError === null;
  const needSyncModal =
    (logsManuallyOpen || currentOperation !== null) && currentError === null;

  return (
    <>
      <SyncingModal
        isVisible={needSyncModal}
        title={currentOperation ?? "Log output"}
        subtext={statusText}
        onClose={canClose ? () => setLogsManuallyOpen(false) : undefined}
      />
      <ErrorModal
        isVisible={currentError !== null}
        title={currentError?.title ?? ""}
        description={currentError?.error}
        onClose={() => setCurrentError(null)}
      ></ErrorModal>
    </>
  );
}

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
