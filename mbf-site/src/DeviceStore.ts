import { Adb } from '@yume-chan/adb';
import { create } from "zustand";
/**
 * Device store is used to save the data about the device and a reference to the ADB connection.
 */

export interface DeviceStore {
    /**
     * The ADB connection to the device.
     */
    device: Adb | null;
    /**
     * Is the device preV51?  (Quest 1)
     */
    devicePreV51 : boolean;
    /**
     * The device name, if available.
     */
    androidVersion: number | null;
    setDevicePreV51: (isPreV51: boolean) => void;
    setAndroidVersion: (version: number | null) => void;
    setDevice: (adb: Adb | null) => void;

}

export const useDeviceStore = create<DeviceStore>(set => ({
    device: null,
    devicePreV51: false,
    androidVersion: null,
    Adb: null,
    setDevicePreV51: (isPreV51: boolean) => set(() => ({ devicePreV51: isPreV51 })),
    setAndroidVersion: (version: number | null) => set(() => ({ androidVersion: version })),
    setDevice: (device: Adb | null) => set(() => ({ device: device }))
}));