import { ReactNode } from "react";
import { type DeviceConnectorCallbacks } from "./hooks/DeviceConnector";
import { AdbServerClient } from "@yume-chan/adb";
import { bridgeData, WebSocketBridge } from "./AdbServerWebSocketConnector";
import { MbfBridge } from "./MbfAdbServerConnector";

export interface IBridge {
    /**
     * A react component that provides supplemental support for the bridge.
     * 
     * This should be rendered within the {@link DeviceConnectorCallbacks.DeviceConnectorContextProvider}.
     * @returns 
     */
    BridgeSupplement(): ReactNode;
    
    isAvailable(AbortController: AbortController): Promise<boolean>;
    
    getConnector(): Promise<AdbServerClient.ServerConnector>;
}



export class BridgeFactory {
    static async getBridge(abortController: AbortController = new AbortController()): Promise<IBridge | void> {
        if (bridgeData) {
            const bridge = new WebSocketBridge(bridgeData);
            
            if (await bridge.isAvailable(abortController)) {
                return bridge;
            }
        }
        
        // Native bridge handling.
        {
            const bridge = new MbfBridge();
            
            if (await bridge.isAvailable(abortController)) {
                return bridge;
            }
        }
    }
}