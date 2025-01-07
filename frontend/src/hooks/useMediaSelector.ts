import {Media} from "@/api/api";
import {useEffect, useState} from "react";

export function useMediaSelector(media: Media[], layout: Media[][] | null) {
    const [target, setTarget] = useState<Media | null>(null);
    const [selected, setSelected] = useState<Media[]>([]);

    const [shiftDown, setShiftDown] = useState<boolean>(false);

    useEffect(() => {
        function handleKeyDown(e: KeyboardEvent) {
            if (e.key === 'Shift') {
                setShiftDown(true);
            }

            if (!target) {
                return;
            }


            if (e.key === 'ArrowLeft'){
                const a = media.findIndex(m => m.uuid === target.uuid);
                const b = Math.max(0, a - 1);
                select(media[b]);
            }

            if (e.key === 'ArrowRight'){
                const a = media.findIndex(m => m.uuid === target.uuid);
                const b = Math.min(media.length - 1, a + 1);
                select(media[b]);
            }

            if (e.key === 'ArrowUp'){
                const a = media.findIndex(m => m.uuid === target.uuid);
                const b = layout ? Math.max(0, a - layout[0].length) : Math.max(0, a - 1);
                select(media[b]);
            }

            if (e.key === 'ArrowDown'){
                const a = media.findIndex(m => m.uuid === target.uuid);
                const b = layout ? Math.min(media.length - 1, a + layout[0].length) : Math.min(media.length - 1, a + 1);
                select(media[b]);
            }
        }

        function handleKeyUp(e: KeyboardEvent) {
            if (e.key === 'Shift') {
                setShiftDown(false);
            }
        }

        window.addEventListener('keydown', handleKeyDown);
        window.addEventListener('keyup', handleKeyUp);

        return () => {
            window.removeEventListener('keydown', handleKeyDown);
            window.removeEventListener('keyup', handleKeyUp);
        };

    }, [target, media, layout, shiftDown]);

    function select(s: Media | null) {
        if (!s) {
            setTarget(null);
            setSelected([]);
            return;
        }


        setTarget(s);

        if (!shiftDown || !target) {
            setSelected([s]);
            return;
        }


        const a = media.findIndex(m => m.uuid === target.uuid);
        const b = media.findIndex(m => m.uuid === s.uuid);

        // we want to add all the media between start and end to the selection
        const [start, end] = [a, b].sort((a, b) => a - b);
        const newSelection = media.slice(start, end + 1);

        const map: {[key: string]: Media} = {};

        selected.forEach(m => map[m.uuid] = m);
        newSelection.forEach(m => map[m.uuid] = m);

        setSelected(Object.values(map));
    }


    return {selected, select, target};

}