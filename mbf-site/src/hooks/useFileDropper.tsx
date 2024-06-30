import { DragEvent, useEffect, useRef, useState } from 'react';

interface Props {
    onFilesDropped?: (e: File[]) => Promise<void>;
    onUrlDropped?: (url: string) => Promise<void>;
}

export default function useFileDropper(props: Props) {
    const [isDragging, setIsDragging] = useState(false);
    const dragCounter = useRef(0);

    useEffect(() => {
        // Make functions for drag events
        function ondragover(e: any) {
            e.preventDefault();
            e.stopPropagation();
        }
        function ondragenter(e: any) {
            e.preventDefault();
            e.stopPropagation();

            const currentCounter = dragCounter.current;
            if (currentCounter + 1 >= 0) {
                setIsDragging(true);
            }

            let newValue = currentCounter + 1;
            // Prevent negative values
            if (newValue < 0) newValue = 0;
            newValue === 0 ? setIsDragging(false) : setIsDragging(true);
            dragCounter.current = newValue;
        }

        function ondragleave(e: any) {
            e.preventDefault();
            e.stopPropagation();

            let newValue = dragCounter.current - 1;
            // Prevent negative values
            if (newValue < 0) newValue = 0;
            if (newValue === 0) setIsDragging(false);
            dragCounter.current = newValue;
        }

        async function ondrop(e: any) {
            e.preventDefault();
            try {
                setIsDragging(false);
                dragCounter.current = 0;
                if (props.onFilesDropped) {
                    const files = getFilesFromDragEvent(e);
                    if (files.length > 0) {
                        await props.onFilesDropped(files);
                    }
                }
                if (props.onUrlDropped) {
                    const url = getUrlFromDragEvent(e);
                    if (url) {
                        await props.onUrlDropped(url);
                    }
                }
            } catch (e) {
                console.error(e);
            }
        }

        window.addEventListener('dragover', ondragover);
        window.addEventListener('drop', ondrop);
        window.addEventListener('dragenter', ondragenter);
        window.addEventListener('dragleave', ondragleave);
        return () => {
            window.removeEventListener('dragover', ondragover);
            window.removeEventListener('drop', ondrop);
            window.removeEventListener('dragenter', ondragenter);
            window.removeEventListener('dragleave', ondragleave);
        }
    }, [])

    return {
        isDragging,
    }
}


function getFilesFromDragEvent(e: DragEvent) {
    // Try 2 ways of getting files
    // If dropped items aren't files, reject them

    // If it's files, process them and send them to the server one by one
    let filesToUpload: Array<File> = [];

    // If no data transfer, return empty array
    if (!e.dataTransfer) return [];

    // Try 2 ways of getting files
    if (e.dataTransfer.items) {
        // Use DataTransferItemList interface to access the file(s)
        for (let i = 0; i < e.dataTransfer.items.length; i++) {
            const item = e.dataTransfer.items[i];
            // If dropped items aren't files, reject them
            if (item.kind === 'file') {
                const file = item.getAsFile();
                if (file) {
                    console.log(`… file[${i}].name = ${file.name}`);
                    filesToUpload.push(file);
                }
            }

        }
    } else {
        // Use DataTransfer interface to access the file(s)
        for (let i = 0; i < e.dataTransfer.files.length; i++) {
            const file = e.dataTransfer.files[i];
            console.log(`… file[${i}].name = ${file.name}`);
            filesToUpload.push(file);
        }
    }
    return filesToUpload;
}


function getUrlFromDragEvent(e: DragEvent) {
    if (!e.dataTransfer) return;

    // Get the url if there is one
    let url = e.dataTransfer.getData('URL');

    if (url) {
        return url;
    }
    return undefined;
}
