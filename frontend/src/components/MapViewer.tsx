import {Media} from "@/api/api";
import Map from "@/components/Map/Map";
import styles from './MapViewer.module.css';
import React, {useCallback, useEffect, useState} from "react";
import {QueryState} from "@/hooks/useQueryState";
import Filter from "@/utility/Filter";
import {MapState} from "@/components/Map/DynamicMap";

export interface MapViewerProps {
    media: Media[] | null;
    select: (media: Media) => void;
    filter: Filter;
    setGalleryState: (state: Partial<QueryState>) => void;
}

export default function MapViewer({media, select, filter, setGalleryState}: MapViewerProps) {
    const [recenterFunction, setRecenterFunction] = React.useState<() => void>(() => () => {});
    const [mapState, setMapState] = useState<MapState | null>(null);


    useEffect(() => {
        if (!filter.get('has_gps', "=")) {
            const newFilter = filter.clone();
            newFilter.set('has_gps', '=', true);
            setGalleryState({filter: newFilter});
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
                centerOnMedia={!filter.filter.hasOwnProperty("longitude") && !filter.filter.hasOwnProperty("latitude")}
                scrollWheelZoom={true}
                setRecenterFunction={setRecenterFunctionWrapper}
                setMapState={setMapState}
            ></Map>
            <div className={styles.buttonOverlay}>
                <div onClick={() => {
                    if(recenterFunction){
                        recenterFunction();
                    }
                }}>Recenter</div>
                <div onClick={() => {
                    if(mapState) {
                        const ne = mapState.bounds.getNorthEast();
                        const sw = mapState.bounds.getSouthWest();

                        const newFilter = filter.clone();
                        newFilter.set("latitude", "<=", ne.lat)
                        newFilter.set("latitude", ">=", sw.lat)
                        newFilter.set("longitude", "<=", ne.lng)
                        newFilter.set("longitude", ">=", sw.lng)
                        setGalleryState({filter: newFilter});
                    }
                }}>Search Here</div>
            </div>
        </div>
    )
}