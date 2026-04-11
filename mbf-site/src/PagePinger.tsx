import { useEffect } from 'react';

interface PagePingerProps {
    url: string;
    interval: number; // Interval in milliseconds
}

/**
 * PagePinger component that pings a specified URL at a given interval.
 * @param props
 * @returns
 */
export function PagePinger({ url, interval }: PagePingerProps) {
    useEffect(() => {
        const intervalId = setInterval(() => {
            fetch(url)
                .then((response) => {
                    if (!response.ok) {
                        console.error(`Failed to fetch ${url}: ${response.statusText}`);
                    }
                })
                .catch((error) => console.error(`Error fetching ${url}:`, error));
        }, interval);

        // Cleanup function to clear the interval when the component is unmounted
        return () => {
            clearInterval(intervalId);
        };
    }, [url, interval]);

    return null; // This component does not render anything
}
