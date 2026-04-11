import { ReactNode } from "react";
import { type DeviceConnectorCallbacks } from "./hooks/DeviceConnector";
import { AdbServerClient } from "@yume-chan/adb";
import { bridgeData, WebSocketBridge } from "./AdbServerWebSocketConnector";

export interface IBridge {
    /**
     * A react component that provides supplemental support for the bridge.
     * 
     * This should be rendered within the {@link DeviceConnectorCallbacks.DeviceConnectorContextProvider}.
     * @returns 
     */
    BridgeSupplement(): ReactNode;
    
    isAvailable(): Promise<boolean>;
    
    getConnector(): Promise<AdbServerClient.ServerConnector>;
}



export class BridgeFactory {
    static async getBridge(): Promise<IBridge | void> {
        if (bridgeData) {
            const bridge = new WebSocketBridge(bridgeData);
            
            if (await bridge.isAvailable()) {
                return bridge;
            }
        }
    }
}