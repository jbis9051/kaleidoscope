import {Media} from "@/api/api";
import Map from "@/components/Map/Map";
import styles from './MapViewer.module.css';
import React, {useCallback, useEffect} from "react";
import {FilterOps, QueryState} from "@/hooks/useQueryState";

export interface MapViewerProps {
    media: Media[] | null;
    select: (media: Media) => void;
    filter: FilterOps;
    setGalleryState: (state: Partial<QueryState>) => void;
}

export default function MapViewer({media, select, filter, setGalleryState}: MapViewerProps) {
    const [recenterFunction, setRecenterFunction] = React.useState<() => void>(() => () => {});

    useEffect(() => {
        if (!filter.has_gps) {
            setGalleryState({filter: {...filter, has_gps: true}});
        }
    }, [filter]);

    const setRecenterFunctionWrapper = useCallback((fn: () => void) => {
        setRecenterFunction(() => fn);
    }, []);

    return (
        <div className={styles.viewer}>
            <Map
                className={styles.map}
                center={[0, 0]}
                zoom={13}
                select={select}
                media={media || []}
                centerOnMedia={true}
                scrollWheelZoom={true}
                setRecenterFunction={setRecenterFunctionWrapper}
            ></Map>
            <div className={styles.recenter} onClick={() => {
                if(recenterFunction){
                    recenterFunction();
                }
            }}>Recenter</div>
        </div>
    )
}