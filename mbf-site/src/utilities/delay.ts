/**
 * Creates a promise that resolves after a specified delay.
 * @param ms - The number of milliseconds to delay.
 * @param signal - Optional AbortSignal to cancel the delay.
 * @returns A promise that resolves after the specified delay.
 */
export function delay(ms: number, signal?: AbortSignal): Promise<void> {
    return new Promise((resolve, reject) => {
        const timeoutId = setTimeout(resolve, ms);
        if (signal) {
            signal.addEventListener('abort', () => {
                clearTimeout(timeoutId);
                reject(new Error('Delay aborted'));
            });
        }
    });
}